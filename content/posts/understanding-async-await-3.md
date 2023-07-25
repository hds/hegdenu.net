+++
title = "how I finally understood async/await in Rust (part 3)"
slug = "understanding-async-await-3"
author = "hds"
date = "2023-07-31"
draft = true
+++

This is a series on how I understood async/await in Rust.

This is part 3.

Here's the full list:

* [part 1: why doesn’t my task do anything if I don’t await it?](@/posts/understanding-async-await-1.md)
* [part 2: how does a pending future get woken?](@/posts/understanding-async-await-2.md)
* [part 3: why shouldn’t I hold a mutex guard across an await point?](#why-shouldn-t-i-hold-a-mutex-guard-across-an-await-point) (this very post)
* part 4: why would I ever want to write a future manually? (not too long, we hope)

Previously we looked at what happens when a future gets awaited.

We also dove into how a pending future gets awoken.

Now we're going back to that await topic.

We're going to look into why some things shouldn't be held across await points.

Often the first thing a new author of async code needs to do is share state.

Typically, shared state is protected by a mutex.

(or some mutex variant, like a read-write lock)

So today we'll focus on mutex guards.

### why shouldn't I hold a mutex guard across an await point?

The first thing that many people try to do in async code is share state.

This is often not a good idea.

(actually, it's often not even necessary)

(but that is a story for another day)

But sometimes it **is** necessary.

And it's worth looking at anyway.

Because what we're interested in here is **understanding** async/await.

Not necessarily doing it right.

(yet)

### aside: mutexes and mutex guards in rust

TODO: explain how mutex guards work in rust

Link [`std::sync::Mutex`](https://doc.rust-lang.org/std/sync/struct.Mutex.html).

### exclusive-value-sleep future

So let's imagine that we've got a use case where we want a mutex held across an await point.

This could be:

1. Read shared counter
2. Access async shared resource (a database handle perhaps?)
3. Write new value to shared counter

I'm sorry I couldn't come up with a more convincing example.

But this will do.

Now, let's replace our async shared resource with an async sleep.

Because we don't actually care about what it is.

Here's our async function.

```rust
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

async fn exclusive_value_sleep(data: Arc<Mutex<u64>>) -> Result<(), DataAccessError> {
    let mut guard = data.lock().map_err(|_| MyError {})?;
    println!("existing value: {}", *guard);

    tokio::time::sleep(Duration::from_millis(10)).await;

    *guard = *guard + 1;
    println!("new value: {}", *guard);

    Ok(())
}
```

Let's run through it.

Our future takes some data.

Actually it's an `Arc<Mutex<u64>>`.

Let's look at this from the inside out.

Our value is a `u64`.

(because life is too short for 32-bit numbers)

We want to access and modify our value from multiple tasks though.

So it's wrapped in a `Mutex`.

We already looked at the basics of [mutexes and mutex guards in rust](#aside-mutexes-and-mutex-guards-in-rust).

Finally, we need to access our mutex from multiple tasks.

So it's wrapped in a [`std::sync::Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html).

An `Arc` is actually an acronym: ARC.

Atomically Reference Counted.

It's a shared pointer.

It can be cloned and passed around between tasks.

(and threads)

All the while, giving access to the same location in memory.

The location where our mutex is!

So what we do is the following.

We lock the mutex.

(now only we have access to it)

We print out the value of our shared data.

Now we "access our async resource".

(actually we sleep for 10 milliseconds)

Then update the value of the shared data.

And we're done!

### that return type

Oh, you may have noticed something.

The return type!

Locking a mutex can fail.

So we should return an error in this case.

It is good practice to make your errors explicit and minimal.

So here we've defined a new error for this case.

It's very simple, and I won't go into it today.

But here it is for reference.

```rust
use std::{error::Error, fmt};

#[derive(Debug)]
struct DataAccessError {}

impl fmt::Display for DataAccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "there was an error accessing the shared data")
    }
}

impl Error for DataAccessError {}
```

If you put all of this into a project, it will compile.

So let's execute it.

### running the exclusive-value-sleep future

Let's call our future.

Or rather await it.

We'll use the `#[tokio::main]` macro for brevity.

But in part 2, we looked at [unwrapping async main()](@/posts/understanding-async-await-2.md#unwrapping-async-main).

We'll probably unwrap it again for great clarity later on.

Our nice simple main function.

```rust
#[tokio::main]
async fn main() {
    let data = Arc::new(Mutex::new(0_u64));

    exclusive_value_sleep(Arc::clone(&data))
        .await
        .expect("failed to perform operation");
}
```

We create our data.

(initial value of 0)

And then we await our future.

Remember: this is a naughty async function.

It's holding a mutex guard across an await point!

So let's see what happens when we await it.

```
existing value: 0
new value: 1
```

It works.

(that is just a little disappointing)

Clearly, we're going to have to try harder to do bad things.

### aside: spawn

We can't just await our async function twice in succession.

Because then it will run successively.

(one time after the other)

However, there is a way to run multiple futures concurrently on an async runtime.

It's usually called `spawn`.

In the case of Tokio, it is called `spawn`, [`tokio::spawn`](https://docs.rs/tokio/latest/tokio/task/fn.spawn.html).

(technically, it's `tokio::task::spawn`)

(but it's aliased at `tokio::spawn` and that's how it's generally used)

A **huge** difference when using `tokio::spawn` is that you don't need to `.await`.

The future will be set to execute immediately in a new task.

In fact, this is how we end up with more than one task.

(I know, we're up to part 3 and we've only just discovered multiple tasks)

Spawn returns a join handle: [`tokio::task::JoinHandle`](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html).

The join handle can be used to wait for the completion of the task.

(the join handle implements `Future` so it can be `.await`ed just like any future!)

It can also be used to **abort** the spawned task.

(which is another story for another day)

Let's get back to trying to break something.

### spawning multiple async functions

Let's spawn a couple of instances of our async function!

```rust
#[tokio::main]
async fn main() {
    let data = Arc::new(Mutex::new(0_u64));

    tokio::spawn(exclusive_value_sleep(Arc::clone(&data)));
    tokio::spawn(exclusive_value_sleep(Arc::clone(&data)));
}
```

Now we'll run it and...

Oh.

It doesn't compile.

And we get a lot of errors.

Many of those errors are duplicates.

Because we called spawn twice.

So we'll comment out one of the spawn lines.

And now the errors are a bit more manageable.

(by the way, the person who writes a syntax highlighter for `rustc` output will be my hero forever)

```
error: future cannot be sent between threads safely
   --> resources/understanding-async-await/src/bin/mutex_guard_async.rs:5:18
    |
5   |     tokio::spawn(exclusive_value_sleep(Arc::clone(&data)));
    |                  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ future returned by `exclusive_value_sleep` is not `Send`
    |
    = help: within `impl Future<Output = Result<(), DataAccessError>>`, the trait `Send` is not implemented for `std::sync::MutexGuard<'_, u64>`
note: future is not `Send` as this value is used across an await
   --> resources/understanding-async-await/src/bin/mutex_guard_async.rs:18:50
    |
15  |     let mut guard = data.lock().map_err(|_| DataAccessError {})?;
    |         --------- has type `std::sync::MutexGuard<'_, u64>` which is not `Send`
...
18  |     tokio::time::sleep(Duration::from_millis(10)).await;
    |                                                  ^^^^^^ await occurs here, with `mut guard` maybe used later
...
24  | }
    | - `mut guard` is later dropped here
note: required by a bound in `tokio::spawn`
   --> /Users/stainsby/.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.27.0/src/task/spawn.rs:163:21
    |
163 |         T: Future + Send + 'static,
    |                     ^^^^ required by this bound in `spawn`

```

OK, it's really just one error.

And two notes.

Let's read through.

The error itself first.

`error: future cannot be sent between threads safely`

A spawned task may be started on any worker (thread).

Even a current-thread runtime may have a task spawn from *another* thread.

So this makes sense that a future **needs** to be able to be sent between threads.

But why can't it?

The first note tells us.

``note: future is not `Send` as this value is used across an await``

It then points us to `mut guard`.

And tells us that it isn't `Send`.

And then points to the `.await` where we sleep as the offending await point.

(rust errors are amazing!)

Finally, the error goes on to tell us that it is all `spawn`'s fault!

``note: required by a bound in `tokio::spawn` ``

This `Send` stuff isn't magic.

It is specified specifically on `tokio::spawn` by the tokio authors.

Let's go an have a look at the code for it.

([`tokio::spawn` from the `tokio-1.29.1` tag](https://github.com/tokio-rs/tokio/blob/1b1b9dc7e388d0619fe7bfe6a7618fff596fdee1/tokio/src/task/spawn.rs#L164-L167))

```rust
pub fn spawn<T>(future: T) -> JoinHandle<T::Output>
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    // (we're skipping the actual implementation)
}
```

We see that spawn is generic over `T`.

And there are some constraints on what `T` can be.

In Rust, these "constraints" are called **bounds**.

(ahhhh, that's what a bound is!)

So we can see that `T` must implement `Future` and `Send` and have a `'static` lifetime.

We're going to skip over the lifetime.

Lifetimes is a whole other series.

That the type must implement `Future` makes sense.

This is the whole point of spawn after all.

And we kind of understand why this future can't be sent between threads.

But how does a type "implement" `Send`?

### aside: marker traits

Rust has a concept called "marker traits".

(you can find them in the standard library [`std::marker`](https://doc.rust-lang.org/std/marker/index.html))

These are traits that don't have any methods.

Mandatory or otherwise.

So they're not implemented in the traditional sense.

But when applied to types, they indicate intrinsic properties.

(intrinsic properties means things about the type by itself)

In the case we're looking at, the `Send` trait indicates that a type can be safely sent between threads.

And all along we thought that the Rust compiler was so clever that it could work that out by itself?

(at least this is what I thought for a long time)

If we look at the [`std::marker::Send`](https://doc.rust-lang.org/std/marker/trait.Send.html) trait more closely, we'll see something.

It's `unsafe`!

Yes, it follows that clever Rust convention.

When the compiler can't work out that something is safe by itself.

(but we know that it's safe)

Then we use `unsafe` to indicate that we, the authors of this code, are vouching for its safety.

(in this case, it's not us, it's the authors of the standard library)

(and if you can't trust them to write safe things...)

By default, a struct will be `Send` if everything in it is `Send`.

So mostly we don't have to worry about specifying things as send.

But we need to be wary of where we can use types that **aren't** send.

### back to trying to break things

Rust's type system has foiled our plans.

Our plans of doing something bad with a mutex guard and an await point.

But we can probably still provoke something bad.

We don't need to run our not `Send` async function twice concurrently.

We just need to try to lock that mutex from somewhere else.

So let's create another async function that we can spawn.

It's the same as the previous one, but without the sleep.

(and therefore, without an await point)

(so it's not really async)

```rust
async fn sleepless_exclusive_value(data: Arc<Mutex<u64>>) -> Result<(), DataAccessError> {
    let mut guard = data.lock().map_err(|_| DataAccessError {})?;
    println!("existing value: {}", *guard);

    *guard = *guard + 1;
    println!("new value: {}", *guard);

    Ok(())
}
```

We're not holding the guard across an await point.

So this async function is `Send`!

We need to make one more change.

To ensure that this breaks.

(because this is all pretty simple)

We're going to use a current-thread runtime.

This means that tasks won't run in parallel.

So it's easier to create certain situations.

```rust
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let data = Arc::new(Mutex::new(0_u64));

    tokio::spawn(sleepless_exclusive_value(Arc::clone(&data)));
    exclusive_value_sleep(Arc::clone(&data))
        .await
        .expect("failed to perform operation");
}
```

Here we spawn our `Send` async function.

And then immediately await our bad function.

Let's check the output.

(yes, it does compile, that's a good start)

```
existing value: 0
```

And then it just hangs there.

Forever.

We've created a deadlock!

(give yourself a pat on the back)

(we got there in the end)

Now, it's time to understand **why**.

So we'll go back to our old tricks.

And write a custom `Future` for our async function.

