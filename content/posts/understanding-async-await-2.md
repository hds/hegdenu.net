+++
title = "how I finally understood async/await in Rust (part 2)"
slug = "understanding-async-await-2"
author = "hds"
date = "2023-06-29"
+++

This is the second part in a series on understanding async/await in Rust.

Or rather, on how **I** understood async/await.

As you're not me, this may or may not help you understand too.

(but I hope it does)

Here's the full list of posts in the series.

* [part 1: why doesn’t my task do anything if I don’t await it?](@/posts/understanding-async-await-1.md)
* [part 2: how does a pending future get woken?](#how-does-a-pending-future-get-woken) (this post right here)
* [part 3: why shouldn’t I hold a mutex guard across an await point?](@/posts/understanding-async-await-3.md) (now available)
* [part 4: why would I ever want to write a future manually?](@/posts/understanding-async-await-4.md) (also available)

In the previous part, we looked at the simplest async function.

An async function so simple that it doesn't do anything async.

Then we wrote a custom future to do the same thing.

Doing this, we understood why our simplest future really **is** async.

Why it doesn't execute the contents until it is `await`ed.

In that exploration, an important part of our future was skipped.

(actually, we skipped a lot of things that will become important)

(but those things weren't important at the time, so skipping was ideal)

Our future only ever returned `Poll::Ready`.

But what about a pending future?

Let's look at what happens when we return `Poll::Pending`

### how does a pending future get woken?

First, let's recap what happens when a future gets polled.

We can create an even simpler future than the [Hello, World](@/posts/understanding-async-await-1.md) one.

### ready future

This future will do nothing except return `Poll::Ready`.

We don't even need any members for this.

So we'll start with a unit `struct` and implement `Future` for it.

```rust
use std::{future::Future, task::Poll};

struct Ready;

impl Future for Ready {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        println!("Ready: poll()");
        Poll::Ready(())
    }
}
```

We won't have a return value, so `Output` is the unit type `()`.

The implementation of the `poll` method is simple.

It returns `Poll::Ready(())`.

(the extra brackets in there is the unit type `()` again)

In [part 1](@/posts/understanding-async-await-1.md) we visualised a state machine of the future we wrote.

Even though the `Ready` future is even simpler, let's check the state machine.

![State machine of the Ready future.](/img/understanding-async-await-2/ready-state_machine.svg)

Here it becomes clear that we don't have states in this future.

Additionally, there is no handling of the future being (incorrectly) polled after returning `Poll::Ready`.

All in all, it's a simple future.

Now let's wrap our future in a function.

```rust
fn ready() -> Ready {
    Ready {}
}
```

(we are returning the `Ready` unit struct that implements `Future`)

(not to be confused with `Poll::Ready`)

Since `Ready` implements the `Future` trait, we can await this function.

(we learned this in [part 1](@/posts/understanding-async-await-1.md))

```rust
#[tokio::main]
async fn main() {
    println!("Before ready().await");
    ready().await;
    println!("After ready().await");
}
```

If we run this, we see the expected output immediately.

```
Before ready().await
Ready: poll()
After ready().await
```

What happens behind the `.await` syntax is that the `poll` function gets called.

As it returned `Poll::Ready`, the result is passed straight back to the caller.

For completeness, here is the sequence diagram for our program using the `Ready` future.

![Sequence diagram for the Ready future.](/img/understanding-async-await-2/ready-sequence_diagram-v1.svg)

This future could be useful in test contexts.

In case you want a future that always returns ready.

In fact, other people think it's useful too.

There's a generic version in the futures crate: [`futures::future::ready`](https://docs.rs/futures/latest/futures/future/fn.ready.html)

But we want to know about **not** returning `Poll::Ready`.

So let's have a look!

### pending future

(I think that **pending future** might be a good name for a band)

Let's try to create an equivalent of the ready future, but pending.

The same as for `Ready`, we'll create a unit `struct`.

This time called `Pending`.

Then we'll implement the `Future` trait for it.

```rust
use std::{future::Future, task::Poll};

struct Pending;

impl Future for Pending {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        println!("Pending: poll()");
        Poll::Pending
    }
}
```

Even though we need to define the associated type `Output`, it isn't used.

This is because when a future returns `Poll::Pending` the return value isn't ready yet.

(that's why we're not returning `Poll::Ready`, because it's *not ready*)

As before, we'll wrap our `Pending` future in a function.

```rust
fn pending() -> Pending {
    Pending {}
}
```

(we are returning the `Pending` unit struct that implements `Future`)

(not to be confused with `Poll::Pending`)

### aside: why do we keep wrapping futures in functions?

You might ask yourself, why do we keep wrapping futures in functions?

(or you might ask me)

This is for two reasons.

Reason one is style.

In this blog series, we're exploring what async/await does under the cover.

So it's nice to compare apples to apples.

(or at least compare functions to functions)

Basically, look at a function that can be `await`ed like an `async` function can be.

Reason two is abstraction.

By constructing the future in our own function, we can hide the details from the user of our API.

We can even go so far as to prevent the construction of our future outside of our own crate or module.

This makes backwards compatibility easier.

We can go further than this.

We don't need to declare that we're returning our type from the function at all.

We could instead return *something* that implements the `Future` trait.

Because the `Future` trait has the associated `Output` type, we need to specify that too.

But that's everything.

Let's rewrite our `pending` function in this way.

```rust
fn pending() -> impl Future<Output = ()> {
    Pending {}
}
```

Now we don't need to make `Pending` public at all!

### back to pending

It doesn't matter which return declaration we use.

(either `Pending` or `impl Future<Output = ()>`)

We can still `.await` on the return value of `pending()`.

So let's start up our async runtime and try it out!

```rust
#[tokio::main]
async fn main() {
    println!("Before pending().await");
    pending().await;
    println!("After pending().await");
}
```

You should read a few of lines ahead before executing this.

(in case you're building everything as we go)

(trust me, it's important)

First, here's the output.

```
Before pending().await
Pending: poll()
```

Don't wait for the program to end.

This program won't end.

It will hang there forever.

It won't use a lot of CPU.

It won't block the execution of the thread.

But it won't go any further.

And what is also clear is that `poll()` only gets called once!

Our future is never polled again after returning `Poll::Pending`.

It's true that this future seems broken in all sorts of ways.

But it can be useful in certain scenarios, like tests.

And just like our `ready()` example, there's a generic version in the `futures` crate: [futures::future::pending](https://docs.rs/futures/latest/futures/future/fn.pending.html).

Back to why `Pending` is hanging our program.

Let's check our state machine.

Maybe the state machine can explain what's happening.

![State machine of the Pending future.](/img/understanding-async-await-2/pending-state_machine.svg)

We used a dotted line to indicate on the path to Final.

This is to indicate that this object will likely never be dropped.

We don't really have a good way to show this on the sequence diagram.

(this is an observation, not based on any knowledge of what is happening)

In the end, the state machine for `Pending` looks a lot like the one for `Ready`.

What about the sequence diagram?

![Sequence diagram for the Pending future.](/img/understanding-async-await-2/pending-sequence_diagram-v1.svg)

This isn't very enlightening either.

Why doesn't our program advance?

From the sequence diagram above, it's not entirely clear.

We see that our future returns `Poll::Pending` to our `async main()` function.

But we don't see the `println!` invocation that follows.

This flow is actually a small lie.

We need to dig in a bit deeper to understand what is happening.

#### unwrapping async main()

Part of that lie is how `async main()` works.

Specifically what the `#[tokio::main]` macro does.

The other part is what `.await` does underneath.

(and of course what `.await` does underneath is what this series is all about)

Let's unwrap `#[tokio::main]` and have a look at what is inside!

```rust
fn main() {
    let body = async {
        println!("Before pending().await");
        pending().await;
        println!("After pending().await");
    };

    return tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(body);
}
```

This was done with Rust Analyzer's `Expand macro recursively` command.

(I removed some clippy allows to simplify)

We can now see that the body of our `async main()` function is actually placed in an `async` block.

Then a new runtime is created and given the `async` block to run.

(we use `block_on` to give the runtime a future and wait until it finishes)

To clarify, an `async` block is also just a future!

We now have a better understanding of what our `async main()` function was actually doing.

So let's update the sequence diagram as well.

![Sequence diagram for the Pending future, this time with the `[tokio::main]` macro unwrapped.](/img/understanding-async-await-2/pending-sequence_diagram-v2.svg)

We now see that it's actually the async runtime that is calling `poll()` on the future which is driving the main task.

(you probably guessed this already)

(but confirmation is nice)

The main future awaits our `Pending` future.

There's something important to note when a future awaits some sub-future which returns `Poll::Pending`.

Then the future **also** returns `Poll::Pending` back to its caller.

In this case that goes back to the async runtime.

When the task being polled returns `Poll::Pending` the task itself goes to sleep.

(it's tired, let the poor thing rest)

The async runtime then picks another task to poll.

(it might poll the same task again if it can be polled)

In order for our task to be polled again, it needs to wake up.

But maybe there are no tasks which are scheduled to be polled.

(scheduled to be polled means awake)

In that case, the async runtime parks the thread until a task gets woken.

So, the big question is: when does a task wake up?

Answer: when the waker wakes it.

(a more tautological answer would be impossible)

It turns out that there is a more important question first.

(well, two questions)

What is a waker?

Where can I get one?

### the waker

When we're talking about a waker, we're talking about [`std::task::Waker`](https://doc.rust-lang.org/std/task/struct.Waker.html).

It's a struct in the standard library.

What do the docs say?

> A `Waker` is a handle for waking up a task by notifying its executor that it is ready to be run.

So now we know, we can use the waker to wake up our task.

(tautological as ever, but it really is that simple)

You call [`wake()`](https://doc.rust-lang.org/std/task/struct.Waker.html#method.wake) or [`wake_by_ref()`](https://doc.rust-lang.org/std/task/struct.Waker.html#method.wake_by_ref) on the waker for a task.

Then the task wakes up and polls the future again.

But where do we get one of these from.

More importantly, where do we get a waker for **our** task.

Remember back to part 1 of this series.

In the section [aside: the easy bits of the Future trait](@/posts/understanding-async-await-1.md#aside-the-easy-bits-of-the-future-trait).

I said the following:

> We also don't need the context for now, so we'll skip that too.

This was in reference to the second parameter to the `poll` function: `cx: &mut Context<'_>`

Well, skipping time is over, we now need to understand the context.

### aside: context

The context is the way that information about the current async task is given to a future.

We are specifically talking about [`std::task::Context`](https://doc.rust-lang.org/std/task/struct.Context.html).

We skipped over it in [part 1](@/posts/understanding-async-await-1.md).

We had no need for it.

But the truth is that the context is not complicated.

Let's read the description straight from the docs.

> Currently, `Context` only serves to provide access to a `&Waker` which can be used to wake the current task.

(that's it?)

(yes, that's it)

In fact, `Context` only has two methods.

The first is `from_waker` which constructs a context from a reference to a waker.

The second is `waker` which takes a reference to the context and returns a reference to the waker.

In reality, I think that the `Context` struct is just forward thinking API design.

(this is my uninformed opinion)

(but there's surely an RFC somewhere that explains the real reason)

It may be that in the future, asynchronous tasks have more context.

Not just the waker.

By wrapping the waker like this, that extension would be possible.

If the `poll` function took the waker as a parameter directly, it wouldn't be.

Now we know what a waker is.

And we know where to get one.

Let's write a future that doesn't hang our asynchronous task forever!

### pending but not forever

We want to write a future that returns `Poll::Pending` but doesn't hang forever.

We're all about easy.

So let's do this the easiest way possible.

We need to make 2 changes to our `Pending` future.

Change 1 is to return `Poll::Pending` only once.

From the second call to `poll()`, we will instead return `Poll::Ready`.

But this by itself isn't enough.

As we've seen, `poll()` won't get called again until the task gets woken.

So change 2 is to wake our task.

And we can do this before we return `Poll::Pending`.

(which is the easiest way)

(this is called a [self wake](https://github.com/tokio-rs/console/blob/2de5b68d1a00a77d03a4817f955f385e494368bd/console-subscriber/src/stats.rs#L65) in `tokio-console`, in case you were wondering)

Yes, this works just fine!

We're going to call this future `YieldNow`.

(for reasons we'll see a little later)

Different to our `Ready` and `Pending` futures, we need some state.

Let's look at the code.

```rust
use std::{future::Future, task::Poll};

struct YieldNow {
    yielded: bool,
}

impl Future for YieldNow {
    type Output = ();

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        println!("YieldNow: poll()");
        if self.yielded == true {
            return Poll::Ready(());
        }

        self.yielded = true;

        cx.waker().wake_by_ref();

        Poll::Pending
    }
}
```

Our `YieldNow` struct has a single field.

This determines whether we've "yielded" yet.

Yielding in this context means returning control to the async runtime.

So "yielding" is really just "returning `Poll::Pending`".

If we've already yielded, we return `Poll::Ready`.

If we haven't, we set `yielded` to `true`.

Then we wake the waker!

And finally return `Poll::Pending`.

But because we've already woken our task, we've indicate that we're ready to be polled again.

So our task won't hang!

As usual, let's wrap our future in a function.

```rust
fn yield_now() -> YieldNow {
    YieldNow { yielded: false }
}
```

Now we can try calling it!

(we'll keep our explicit runtime creation)

```rust
fn main() {
    let body = async {
        println!("Before yield_now().await");
        yield_now().await;
        println!("After yield_now().await");
    };

    return tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(body);
}
```

Now we get the desired output immediately.

```
Before yield_now().await
YieldNow: poll()
YieldNow: poll()
After yield_now().await
```

No more hanging!

And we can clearly see that `poll()` gets called twice on `YieldNow`.

We've written our first future with a waker.

Definitely time to celebrate!

### Yield Now

As I mentioned above, we call returning control to the runtime yielding.

This is what happens at every `await` point that returns pending.

(remember that when a future `await`s another future and receives `Poll::Pending` it **also** returns `Poll::Pending`)

(if you have a custom future calling `poll()` directly, this may not be the case)

Our `yield_now()` function is **voluntarily** yielding control to the runtime.

It's voluntarily because the task isn't actually waiting for anything.

The task could otherwise keep progressing.

The name isn't mine.

(I "borrowed" it)

There is a function to do this in Tokio: [`tokio::task::yield_now`](https://docs.rs/tokio/latest/tokio/task/fn.yield_now.html).

(although the tokio implementation is a little more complicated)

(but we can skip that complicatedness for now)

Let's have a look at the state machine for `YieldNow`.

![State machine of the YieldNow future.](/img/understanding-async-await-2/yield_now-state_machine.svg)

Here we include the `poll()` return value in the transition.

The future starts with `yielded = false`.

The first time it is polled, it returns `Poll::Pending()` and transitions to `yielded = true`.

From there, the future will return `Poll::Ready(())` from any further calls to `poll()`.

This state machine is no more complicated than the `HelloWorld` future from [part 1](@/posts/understanding-async-await-1.md).

The more interesting part is the sequence diagram.

So let's check it out.

![Sequence diagram for the YieldNow future.](/img/understanding-async-await-2/yield_now-sequence_diagram.svg)

The `YieldNow` future is very similar to the `Pending` future.

Until it calls `wake_by_ref()` on the waker.

(we saw this function when we introduced [the waker](#the-waker))

The waker then calls to the async runtime to `schedule()` the current task.

(as always, this sequence is logically correct and optimised for understanding)

(it is not exactly matching what happens internally in Tokio)

Now the task is scheduled.

And so we see a difference when the task returns `Poll::Pending` back to the runtime.

The runtime now **does** have a task ready to poll (scheduled).

So it doesn't park the thread.

Instead it polls the task again straight away.

This time, our `YieldNow` future returns `Poll::Ready`.

Since the task that we called `block_on` with is finished, the runtime returns control to `main()`.
 
And it returns the value from our future.

In this case there is no value, so it returns the unit type.

And now we understand how a pending future gets woken!

This post is part of a series.

And [part 3](@/posts/understanding-async-await-3.md) is available to read right now.

### thanks

A huge thank-you to [arriven](https://blog.arriven.wtf/), [Conrad Ludgate](https://github.com/conradludgate), and [sak96](https://sak96.github.io/) for reviews and suggestions!

(in alphabetical order)