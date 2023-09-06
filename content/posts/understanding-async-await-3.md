+++
title = "how I finally understood async/await in Rust (part 3)"
slug = "understanding-async-await-3"
author = "hds"
date = "2023-07-31"
+++

This is a series on how I understood async/await in Rust.

This is part 3.

Here's the full list:

* [part 1: why doesn’t my task do anything if I don’t await it?](@/posts/understanding-async-await-1.md)
* [part 2: how does a pending future get woken?](@/posts/understanding-async-await-2.md)
* [part 3: why shouldn’t I hold a mutex guard across an await point?](#why-shouldn-t-i-hold-a-mutex-guard-across-an-await-point) (this very post)
* [part 4: why would I ever want to write a future manually?](@/posts/understanding-async-await-4.md) (available)

Previously we looked at what happens when a future gets awaited.

We also dove into how a pending future gets awoken.

Now we're going back to that await topic.

We're going to look into why some things shouldn't be held across await points.

Often the first thing a new author of async code needs to do is share state.

Typically, shared state is protected by a mutex.

(or some mutex variant, like a read-write lock)

So today we'll focus on mutex guards.

### why shouldn't I hold a mutex guard across an await point?

Let's assume we want to share state in async code.

This is often not a good idea.

(actually, it's often not even necessary)

(but that is a story for another day)

But sometimes it **is** necessary.

And it's worth looking at anyway.

Because what we're interested in here is **understanding** async/await.

Not necessarily doing it right.

(yet)

### aside: mutexes and mutex guards in rust

We're going to be talking about mutexes a lot.

So it's probably worth going over the basics quickly.

To make sure we're on the same page.

Mutex is short for "Mutual Exclusion".

It's a concurrent programming primitive.

It ensures that only one part of the program is doing some specific thing at a given time.

Usually this is accessing an object which is shared across threads.

(if you don't have multiple threads, then you don't need this sort of protection)

A traditional mutex has two methods.

(traditional here means different to Rust)

Lock.

Unlock.

The code locks the mutex, does something, then unlocks the mutex.

If some other part of the program already holds the mutex, then your code blocks on the lock method.

We could imagine this flow in Rust by inventing our own types.

```rust
// These are NOT real Rust types, especially `MyMutex`
fn exclusive_access(mutex: &MyMutex, protected: &MyObject) {
    // Blocks until a lock can be obtained
    mutex.lock();

    // This "something" is what is protected by the mutex.
    protected.do_something();

    // Once unlocked, other threads can lock the mutex. Don't forget this bit!
    mutex.unlock();
}
```

The problem here is that `MyObject` is only protected by convention.

We have to trust that everywhere that `MyObject` is accessed, the same mutex is locked.

(if you lock a different mutex it really doesn't help at all)

And you might forgot to unlock the mutex when you're finished.

That doesn't seem likely in this toy example.

But imagine that we use the `?` operator to return early from `do_something()` if it returns an error.

Oops!

Now we can't ever lock the mutex again.

#### how rust does mutexes

(by the way, my English speaking brain keeps wanting the plural to be *mutices*)

Instead, Rust sticks the mutex and the object it protects together.

This is [`std::sync::Mutex`](https://doc.rust-lang.org/std/sync/struct.Mutex.html).

The protected object is effectively inside the mutex.

When you lock the mutex you get a `MutexGuard`.

(now we're getting to the mutex guard)

And you can access your protected object through the guard by dereferencing it.

When the guard goes out of scope, the mutex is unlocked automatically.

This behaviour is called RAII.

Which stands for "Resource Acquisition Is Initialization".

For our purposes, it means that we can only get a `MutexGuard` if we can acquire a lock.

(the guard is the resource or 'R' in RAII)

And that the lock is tied to the lifetime of the guard returned.

Once the guard gets dropped, the lock is released.

And therefore, as long as the object is not leaked, the mutex will never stay locked forever.

Wow, that was a lot to take in.

Let's look at our previous example oxidised.

(oxidised as in using Rust standard library and conventions)

```rust
// This is now the std library mutex
fn exclusive_access(mutex: &std::sync::Mutex<MyObject>) {
    // Blocks until a lock can be obtained (same as before)
    let guard = mutex
        .lock()
        .expect("the mutex is poisoned, program cannot continue");

    // The guard gets automatically dereferenced so we can call
    // `MyObject`'s methods on it directly.
    guard.do_something();

    // That's it, once the guard goes out of scope, the lock is released.
}
```

See how we can't accidentally access `MyObject` without locking the mutex.

The type system makes it impossible.

(there are no methods on `std::sync::Mutex` which give access)

And we can't accidentally forget to unlock the mutex either.

Once the guard is dropped, the mutex gets unlocked.

Except...

(there is always an except...)

If the thread holding the mutex guard panics.

Then, when the guard is being dropped.

(since panics don't prevent drops)

Rather than Mutex being unlocked, is _poisoned_.

(you might have seen that mentioned in the code above)

We won't go further into mutex poisoning.

But if you're interested, check the [Mutex documentation](https://doc.rust-lang.org/std/sync/struct.Mutex.html#poisoning) for more information.

#### mutex sequence diagram

Let's try to visualise two threads accessing our protected object.

I've simplified some things.

For example, we can't actually just pass a mutex when spawning 2 threads.

We need to wrap it in a smart pointer.

But we'll see how to do that later.

And it's not important for the purpose of this example.

![Sequence diagram showing two threads accessing the mutex protected `MyObject`.](/img/understanding-async-await-3/mutex-sequence_diagram.svg)

Here we can see how two threads try to lock the mutex.

Only Thread 1 succeeds.

So Thread 2 is parked.

(it stops executing)

Once Thread 1 drops the mutex guard, the mutex is unlocked.

Then Thread 2 can obtain the lock and do its work.

Now, let's get back to using that `MutexGuard` an an async context.

(and by using, I mean using it wrong)

### hold-mutex-guard async function

So let's imagine that we've got a use case where we want a mutex held across an await point.

This could be:

1. Read shared counter
2. Access async shared resource (a database handle perhaps?)
3. Write new value to shared counter

I'm sorry I couldn't come up with a more convincing example.

But this will do.

Now, let's replace our async shared resource with yielding back to the runtime.

Because we don't actually care about what it is.

(this will make things simpler later on)

Here's our async function.

```rust
use std::sync::{Arc, Mutex};

async fn hold_mutex_guard(data: Arc<Mutex<u64>>) -> Result<(), DataAccessError> {
    let mut guard = data.lock().map_err(|_| DataAccessError {})?;
    println!("existing value: {}", *guard);

    tokio::task::yield_now().await;

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

(this is the bit we skipped over in the aside section)

So it's wrapped in a [`std::sync::Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html).

An `Arc` is actually an acronym: ARC.

Atomically Reference Counted.

It's a shared pointer.

It can be cloned and passed around between tasks.

(and threads)

All the while, giving access to the same location in memory.

The location where our mutex is!

Cloning an `Arc` just increments the reference counter.

(atomically in this case)

(the non-atomic version of Arc is [`std::rc::Rc`](https://doc.rust-lang.org/std/rc/struct.Rc.html))

It doesn't clone the underlying data, which is what we want.

(remember that bit about locking a different mutex not being useful)

So what we do is the following.

We lock the mutex.

(now only we have access to it)

We print out the value of our shared data.

Now we "access our async resource".

(actually we just yield back to the runtime)

(we looked at [`yield_now` in part 2](@/posts/understanding-async-await-2.md#yield-now))

Then update the value of the shared data.

And print that out.

And we're done!

#### clippy is watching

It's worth mentioning that while this code compiles, `clippy` doesn't like it.

There is a [lint](https://rust-lang.github.io/rust-clippy/master/#/await_holding_lock) for holding a mutex guard across an await point.

So turn on Clippy lints!

We're not going to run Clippy though.

Because we like living dangerously.

(mostly because we're trying to do the wrong thing)

### that return type

You may have noticed something.

The return type!

Locking a mutex can fail.

(remember poisoning)

So we should return an error in this case.

It is good practice to make your errors explicit and minimal.

So here we've defined a new error for this case.

It's very simple, and I won't go into it.

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

### running the hold-mutex-guard async function

Let's call our future.

Or rather await it.

We'll use the `#[tokio::main]` macro for brevity.

In part 2, we looked at [unwrapping async main()](@/posts/understanding-async-await-2.md#unwrapping-async-main).

We'll probably look at unwrapping it for great clarity later on.

For now, we have a nice simple main function.

```rust
#[tokio::main]
async fn main() {
    let data = Arc::new(Mutex::new(0_u64));

    hold_mutex_guard(Arc::clone(&data))
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

It's called `spawn`, like the function to create a new thread.

In the case of Tokio, it is [`tokio::spawn`](https://docs.rs/tokio/latest/tokio/task/fn.spawn.html).

(technically, it's `tokio::task::spawn`)

(but it's aliased at `tokio::spawn` and that's how it's generally used)

A **huge** difference when using `tokio::spawn` is that you don't need to `.await`.

The future will be set to execute immediately in a new task.

In fact, this is how we end up with more than one task.

(I know, we're up to part 3 and we've only just discovered multiple tasks)

**But.**

The new task may not get polled immediately.

It depends on how occupied the async runtime's workers are.

Let's create a simple example.

We'll use async/await syntax for brevity.

```rust
// `flavor`` has to be one of these values, not both. This code won't compile.
#[tokio::main(flavor = "current_thread|multi_thread")]
async fn main() {
    tokio::spawn(spawn_again());
    do_nothing().await;

    tokio::task::yield_now().await
    tokio::task::yield_now().await

    // ... Let's pretend there's more here and we're not returning yet.
}

async fn spawn_again() {
    tokio::spawn(do_nothing());
}

async fn do_nothing() {
    // There's nothing here
}
```

Here our `async main` function spawns a task with `spawn_again`

(an async function which will spawn another task)

And then it awaits an async function `do_nothing`.

(which does nothing)

The async function `spawn_again` spawns a task with `do_nothing`.

Let's see how this might work with different runtime schedulers.

#### spawn onto current-thread

An async runtime may only have one worker.

For example the [current-thread scheduler](https://docs.rs/tokio/latest/tokio/runtime/index.html#current-thread-scheduler) in Tokio.

Then we could spawn a task from within another task.

But it wouldn't get polled until the current task yields to the scheduler.

(or maybe later if other tasks are waiting)

This is how it would look as a sequence diagram.

![Sequence diagram representing the 3 futures in the code above being polled by a current thread scheduler.](/img/understanding-async-await-3/spawn_current_thread-sequence_diagram.svg)

Note how the tasks that get spawned need to wait until the runtime is free.

Then they will get polled.

But when a task `.await`s a future, there is no new task.

And it gets polled immediately.

#### spawn onto multi-thread

Instead, a runtime may have multiple workers.

(which means multiple threads)

Like the [multi-thread scheduler](https://docs.rs/tokio/latest/tokio/runtime/index.html#multi-thread-scheduler) in Tokio.

Then there can be as many tasks being polled in parallel as there are workers.

Let's take a runtime with 2 workers and see how that would look as a sequence diagram.

Note that there is now parallelism.

So the exact order of operations may vary.

![Sequence diagram representing the 3 futures in the code above being polled by a multi-thread scheduler.](/img/understanding-async-await-3/spawn_multi_thread-sequence_diagram.svg)

This diagram contains a bit of a lie concerning how Tokio works.

Tasks are actually spawned onto the same worker that the spawning task is running on.

If another worker is idle, it may steal tasks from the first worker's queue.

(but all this is out of scope, so we'll continue)

#### wait for me to finish

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

    tokio::spawn(hold_mutex_guard(Arc::clone(&data)));
    tokio::spawn(hold_mutex_guard(Arc::clone(&data)));
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
5   |     tokio::spawn(hold_mutex_guard(Arc::clone(&data)));
    |                  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ future returned by `hold_mutex_guard` is not `Send`
    |
    = help: within `impl Future<Output = Result<(), DataAccessError>>`, the trait `Send` is not implemented for `std::sync::MutexGuard<'_, u64>`
note: future is not `Send` as this value is used across an await
   --> resources/understanding-async-await/src/bin/mutex_guard_async.rs:15:29
    |
12  |     let mut guard = data.lock().map_err(|_| DataAccessError {})?;
    |         --------- has type `std::sync::MutexGuard<'_, u64>` which is not `Send`
...
15  |     tokio::task::yield_now().await;
    |                             ^^^^^^ await occurs here, with `mut guard` maybe used later
...
21  | }
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

(this would mean that the task is spawned from outside the runtime, which is just fine)

So this makes sense that a future **needs** to be able to be sent between threads.

But why can't it?

The first note tells us.

``note: future is not `Send` as this value is used across an await``

It then points us to `mut guard`.

And tells us that it isn't `Send`.

And then points to the `.await` where we yield as the offending await point.

(rust errors are amazing!)

Finally, the error goes on to tell us that it is all `spawn`'s fault!

``note: required by a bound in `tokio::spawn` ``

This `Send` stuff isn't magic.

It is specified explicitly on `tokio::spawn` by the tokio authors.

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

(ohhhh, that's what a bound is!)

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

It's the same as the previous one, but without the yield.

(and therefore, without an await point)

(so it's not really async)

```rust
async fn yieldless_mutex_access(data: Arc<Mutex<u64>>) -> Result<(), DataAccessError> {
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

    tokio::spawn(yieldless_mutex_access(Arc::clone(&data)));
    hold_mutex_guard(Arc::clone(&data))
        .await
        .expect("failed to perform operation");
}
```

Here we spawn our `Send` async function `yieldless_mutex_access()`.

And then immediately await our bad function `hold_mutex_guard()`.

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

### hold-mutex-guard future

We're going to manually implement a future that does the same.

I almost didn't manage this one.

(in fact, I **didn't** manage it)

(luckily I know some smart people who helped me)

([thank you!](#thanks))

Now, on to that future.

Futures are generally implemented as state machines.

(we've seen this a few times before)

We'll need an initial state.

(before being polled)

And we like to have an explicit completed state.

(which will panic if polled again)

And in the middle, a state after having yielded once.

With that in mind, our future could look like the following.

```rust
use std::sync::{Arc, Mutex};

enum HoldMutexGuard<'a> {
    Init {
        data: Arc<Mutex<u64>>,
    },
    Yielded {
        guard: MutexGuard<'a, u64>,
        _data: Arc<Mutex<u64>>,
    },
    Done,
}
```

Our initial state needs the parameters that the async function receives.

The yielded state is going to have our guard stored in it.

(this is the bit we're doing wrong, of course)

We also need the Arc containing our data.

This matches what our async function would have had generated.

(more on why later)

The `MutexGuard` requires a lifetime generic parameter.

(which is a total pain by the way)

(but that's the point, it's there for a good reason)

That means that our future will also need a lifetime generic parameter.

We'll wrap the soon-to-be-future up in a function.

([why? see part 2](@/posts/understanding-async-await-2.md#aside-why-do-we-keep-wrapping-futures-in-functions))

```rust
fn hold_mutex_guard(
    data: Arc<Mutex<u64>>,
) -> impl Future<Output = Result<(), DataAccessError>> {
    HoldMutexGuard::Init { data }
}
```

We're using the same error type too.

Before we implement anything, let's pause.

And take a look at the state machine for `HoldMutexGuard`.

![State machine of the HoldMutexGuard future.](/img/understanding-async-await-3/hold_mutex_guard-state_machine.svg)

It's not much more complicated than [`YieldNow`'s state machine](@/posts/understanding-async-await-2.md#yield-now).

The future starts in the `Init` state.

When polled the first time, it returns `Poll::Pending`.

And moves to the `Yielded` state.

When polled the second time, it returns `Poll::Ready`.

And moves to the `Done` state.

The implementation is a little more complex though.

### implementing the hold-mutex-guard future

Now onto the good stuff.

Implementing `Future`.

```rust
impl<'a> Future for HoldMutexGuard<'a> {
    type Output = Result<(), DataAccessError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let state = &mut *self;
        match state {
            Self::Init { data } => {
                let guard = unsafe {
                    // SAFETY: We will hold on to the Arc containing the mutex as long
                    //         as we hold onto the guard.
                    std::mem::transmute::<MutexGuard<'_, u64>, MutexGuard<'static, u64>>(
                        data.lock().map_err(|_| DataAccessError {})?,
                    )
                };
                println!("existing value: {}", *guard);

                cx.waker().wake_by_ref();
                *state = Self::Yielded {
                    guard: guard,
                    _data: Arc::clone(data),
                };

                Poll::Pending
            }
            Self::Yielded { guard, _data } => {
                println!("new value: {}", *guard);

                *state = Self::Done;

                Poll::Ready(Ok(()))
            }
            Self::Done => panic!("Please stop polling me!"),
        }
    }
}
```

It's not as bad as it looks!

Our `Output` associated type is the same as our function return parameter.

That's easy.

So let's look at the implementation for `poll()`.

Wait, wait, wait.

What is this beast?

```rust
let state = &mut *self;
```

The borrow checker has lots of fun with anything to do with `Pin`.

(but we're still not going to discuss pinning today)

We need to modify `self`.

But it's pinned.

And we need to reference parts of it as well.

So we dereference our pinned self.

Then take a mutable reference.

(this is all legit and the borrow checker is happy)

The first time we get polled, we'll be in the state `Init`.

So we'll do everything up to the `yield_now` call in our async function.

#### a little bit of unsafe

Unfortunately we come up against the borrow checker again.

We can't just take our `MutexGuard` and store it next to the `Mutex` it's guarding.

That would create a self-referential structure.

And Rust is all against those.

In fact it's so against those that we have to use `unsafe` to do what we want.

(admittedly, what we're trying to do is wrong from the start)

(so it's not that surprising)

What we're going to do is create a `MutexGuard` with a `'static` lifetime.

That means, we're telling the borrow checker that it will last as long as it needs to.

In our case, this is legitimately OK.

This is why we keep the Arc stored even though we don't need it.

As long as we hold that Arc to the `Mutex`, the `MutexGuard` can be valid.

To do this magic, we use [`std::mem::transmute`](https://doc.rust-lang.org/std/mem/fn.transmute.html).

(it's alchemy!)

This will reinterpret the bits of one value as another.

This allows us to take the `MutexGuard` with some other lifetime.

And turn it into (transmute) a `MutexGuard` with a static lifetime.

If this doesn't make too much sense, don't worry.

It's not necessary to understand the rest.

But keep in mind that Rust wants to protect us here.

And we are very carefully going around that protection.

(here be dragons, don't do this at home, etc.)

#### holding onto that guard

Once we have our `MutexGuard`, we print the value.

We're now going to yield back to the runtime.

So just like in our `YieldNow` future, we need to [wake our waker](@/posts/understanding-async-await-2.md#the-waker) first.

Otherwise our future will never be polled again.

Then we set the next state: `Yielded`.

(using that funny `&mut *self`)

And return `Poll::Pending`.

The next time our future gets polled, we are already in state `Yielded`.

We will print the value from the `MutexGuard`.

Then move to state `Done` and return `Poll::Ready`.

At that point, the `MutexGuard` will get dropped.

That's the end of the implementation.

The important bit here is that in the `Yielded` state, we hold on to the `MutexGuard` **and return**.

This is what our async function is doing too.

But we don't see it so clearly.

We just see `.await`.

But every time your async function contains an await point, that is the future returning.

(potentially)

And before returning, it has to store all the in-scope local variables in itself.

### hanging around again

Let's reproduce that hanging program again with our future.

Just to make sure we can.

We're going to spawn the same async function to help provoke the hang as we did before.

That's `yieldless_mutex_access` as described in [back to trying to break things](#back-to-trying-to-break-things).

(the one that doesn't actually do anything async)

(we're mixes paradigms a bit, but implementing this future isn't interesting)

And we'll [unwrap async main()](@/posts/understanding-async-await-2.md#unwrapping-async-main) straight away.

(I told you we would get to this)

This leaves us with an unwrapped version of the same code we used before.

```rust
fn main() {
    let body = async {
        let data = Arc::new(Mutex::new(0_u64));

        tokio::spawn(yieldless_mutex_access(Arc::clone(&data)));
        hold_mutex_guard(Arc::clone(&data))
            .await
            .expect("failed to perform operation");
    };

    return tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build runtime")
        .block_on(body);
}
```

We're creating a current-thread runtime.

Same as before.

(this makes triggering hanging behaviour easier)

(and we do like easy)

Let's have a look at the sequence diagram!

Because that's going to help us see what's happening.

![Sequence diagram of the HoldMutexGuard future.](/img/understanding-async-await-3/hold_mutex_guard-sequence_diagram.svg)

The important point is the two futures.

`yieldless_mutex_access()` gets spawned first.

Then `HoldMutexGuard` gets awaited.

As we saw when we introduced [spawn](#aside-spawn), the new task has to wait.

The runtime is single threaded.

So the new task created with `yieldless_mutex_access()` must wait until the current task yields to the runtime.

This means that the `HoldMutexGuard` future is run first.

It locks the mutex and receives a `MutexGuard`.

It wakes it's waker.

(so it will get polled again after returning `Poll::Pending`)

Then changes state to `Yielded`, storing the `MutexGuard` in itself.

And then returns `Poll::Pending`.

Yielding to the runtime.

Now the runtime can poll the next task.

The one spawned with `yieldless_mutex_access()`.

This task locks the mutex.

Well, it tries.

But the mutex is already locked, so it blocks until it gets unlocked.

Since the runtime only has one thread, this blocks the entire runtime.

And causes a deadlock.

We saw this before with our async function.

And now we understand why!

### now what?

So, what **should** we do if we want to control access to some shared async resource?

The obvious answer is to use the async mutex in tokio.

It's called [`tokio::sync::Mutex`](https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html).

It is safe to hold this mutex's guard across await points.

This is because its [`lock()`](https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html#method.lock) method is async.

So it won't block the thread while waiting for the lock.

And so some other task holding the lock can make progress.

(and release the lock)

However, it is often better not to use a mutex at all.

Instead, give full ownership of your shared resource to a single task.

And communicate with that task via [message passing](https://docs.rs/tokio/latest/tokio/sync/index.html#message-passing).

This is a topic for a whole other blog post though.

So we won't go into it here.

In [part 4](@/posts/understanding-async-await-4.md), we look at message passing and channels.

You can go and read it now!

See you next time!

### thanks

A huge thank-you to [Conrad Ludgate](https://github.com/conradludgate) and [Predrag Gruevski](https://predr.ag/) for help in writing the manual future (especially that `MutexGuard` transmute). This post would have been cut short without that.

Twelve points go [Daniel "Yandros" Henry-Mantilla](https://github.com/danielhenrymantilla) for pointing out that `drop()` **does** get called during a panic. This is detected by the `MutexGuard` and used to poison the Mutex. This was submitted as the first ever [PR](https://github.com/hds/hegdenu.net/pull/1) on my [web-site repo](https://github.com/hds/hegdenu.net)!

Thanks to [sak96](https://sak96.github.io/) for reminding me that there is a lint for holding a guard across an await point.

(in alphabetical order - sort of)
