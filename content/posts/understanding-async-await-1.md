+++
title = "how I finally understood async/await in Rust (part 1)"
slug = "understanding-async-await-1"
author = "hds"
date = "2023-05-30"
+++

Like an increasing number of Rustaceans, I came to Rust after async/await had been stabilised.

(strictly speaking this isn't true, but close enough)

I didn't understand what was happening behind those magic keywords.

`async`

`.await`

This made it hard for me to grasp why I had to do things in a certain way.

I wanted to share how I finally understood async Rust.

We'll do this via a series of questions that I had.

We'll explore them in a series of posts.

* [part 1: why doesn’t my task do anything if I don’t await it?](#why-doesn-t-my-task-do-anything-if-i-don-t-await-it) (you're reading it)
* part 2: how does a pending future get woken? (coming soon)
* part 3: why shouldn’t I hold a mutex guard across an await point? (coming less soon)
* part 4: why would I ever want to write a future manually? (coming a bit later)

These were originally going to all go in one post.

But it turned out to be a bit big.

Also, these topics might change order.

Or I might add more.

But first, I'll answer the question on everyone's mind...

### why are you writing another beginners guide to async/await?

There are many different beginner guides on async Rust available.

Why am I writing yet another one?

Personally, nothing I read about async/await made everything click.

This is common.

Different people learn things in different ways.

Maybe no one has written the guide that allows you to understand.

Or maybe there's some piece you don't quite get.

Then reading one more guide fills in that gap so that you do get it.

This post describes how I finally understood.

If it helps you get there, great.

If it doesn't, maybe it's a stepping stone on the way.

Some other guides that I particularly liked are:

* [Let's talk about this async](https://conradludgate.com/posts/async) by Conrad Ludgate
* [Pin and suffering](https://fasterthanli.me/articles/pin-and-suffering) by Amos (fasterthanlime)

### why doesn’t my task do anything if I don’t await it?

Let's get stuck in.

Perhaps the first hurdle newcomers to async Rust meet is that nothing happens.

Here's an example.

```rust
tokio::time::sleep(std::time::Duration::from_millis(100));
```

This will not sleep.

The compiler will immediately warn you that this won't work.


```text
   = note: futures do nothing unless you `.await` or poll them
```

Right, let's fix it.

```rust
tokio::time::sleep(std::time::Duration::from_millis(100)).await;
```

Now we're sleeping!

That's all well and good.

But why?

Normally when I call a function, the contents get executed.

What is so special about that `async` keyword that all my previous experience must be thrown away?

To answer this question, let's look at perhaps the simple async function.

(the simplest that does something)

Generally, guides start with an async function that calls some other async function.

This is simple.

But we want simpler.

We want, hello world.

#### the simplest async function

We're going to start with an async function that doesn't do anything async.

This might seem silly.

But it will help us answer our question.

```rust
async fn hello(name: &'static str) {
    println!("hello, {name}!");
}
```

We can then call this function in a suitable async context.

```rust
#[tokio::main]
async fn main() {
    hello("world").await;
}
```

Our output is as expected.

```text
hello, world!
```

But what does this function actually do.

We know that if we remove the `.await`, nothing happens.

But why?

Let's write our own future that does this.

Specifically, we'll be implementing the trait [std::futures::Future](https://doc.rust-lang.org/std/future/trait.Future.html).

What's a future?

#### aside: futures

A future represents an asynchronous computation.

It is something you can hold on to until the operation completes.

The future will then generally give you access to the result.

It's a common name for this concept in many different programming languages.

The name was proposed in 1977 apparently, so it's not new.

([read Wikipedia for more gory details](https://en.wikipedia.org/wiki/Futures_and_promises))

What we need to know is that a future is what you give an async runtime.

You do this by `.await`ing it.

Then the runtime gives you back the result.

#### the simplest future

Let's write our simple async function as a future.

A future generally has multiple states.

In fact, most futures are "basically" state machines.

The state machine is driven through it's states by the async runtime.

At a minimum we want 2 states, we'll call them `Init` and `Done`

The `Init` state is what the future starts in.

The `Done` state is where the future goes once it's complete.

That's simple.

So we'll model our future as an `enum`.

In the `Init` state, we need to keep the parameters that would be passed to the async function.

```rust
enum Hello {
    Init { name: &'static str },
    Done,
}
```

This isn't a future yet, so we can't await it.

To fix that, we need to implement the Future trait.

#### aside: the easy bits of the Future trait

We're going to just look at the easy bits of the [`std::futures::Future`](https://doc.rust-lang.org/std/future/trait.Future.html) trait.

First, let's look at the trait:

```rust
pub trait Future {
    type Output;

    // Required method
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}
```

The `Future` trait has an associated type that defines the output.

This is what an async function would return.

Our hello function doesn't return anything, so let's skip it for now.

There is one required method that takes a mutable reference to `self` and a context.

The reference to `self` is pinned.

We don't need to understand pinning for now.

So just think of it like any other `&mut self`.

We also don't need the context for now, so we'll skip that too.

The `poll` method returns a [`std::task::Poll`](https://doc.rust-lang.org/std/task/enum.Poll.html) enum.

The method should return `Pending` if it still has work to do.

(there are other things needed when returning `Pending`)

(but we can - yep, you guessed it - skip them for now)

When the future has a value to return, it returns `Ready(T)`.

Here, `T` is the `Future`'s associate type `Output`.

We don't actually need a value here either, so if you don't understand `T`, don't worry.

Skipping over the hard bits makes this easier.

#### implementing poll

Let's look at the implementation.

```rust
use std::{futures::Future, pin::Pin, task::Context};

impl Future for Hello {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        match *self {
            Hello::Init { name } => println!("hello, {name}!"),
            Hello::Done => panic!("Please stop polling me!"),
        };

        *self = Hello::Done;
        Poll::Ready(())
    }
}
```

Let's go through this bit by bit.

Our async function doesn't return a value.

This means that `Output` is the unit type `()`.

This is, in reality, also what our async function returns.

Onto the implementation of `poll`.

We match on `*self`.

(remember, `Hello` is an enum)

If we're in the initial state `Init` then print out `hello, {name}!`.

This is the body of our async function.

If we're in the `Done` state, we panic.

(more on this shortly)

After our match statement, we set our state to `Done`.

Finally, we return `Ready(())`.

(that means `Ready` with a unit type as the value)

(remember that a function that doesn't return anything, actually returns the unit type)

In a moment, we'll look at how to use our new future.

But first, we have a couple of topics pending.

(pun absolutely, 100%, intended)

What about `Poll::Pending` and what about that `panic!`.

#### pending futures

This future is very simple.

It will become ready on the first poll.

But what if that isn't the case?

That's where `Poll::Pending` is used.

We'll look at how to use `Pending` at a later date.

#### future panics

Wait!

What about that panic?

A future is a "one shot" object.

Once it completes - returns `Ready(T)` - it must never be called again.

This is described in the [Panics](https://doc.rust-lang.org/std/future/trait.Future.html#panics) section of the documentation for this trait.

The trait doesn't require that the future panic.

But it's good practice when you start out, as it will quickly catch some logic errors.

#### using our future

We need to construct our new future to be able to use it.

Let's wrap it up in a function like our async function.

```rust
fn hello(name: &'static str) -> impl Future<Output = ()> {
    Hello::Init { name }
}
```

The first thing we note about this function is that it isn't marked `async`.

Because we're returning a "hand made" future, we can't use the `async` keyword.

Instead we return `impl Future<Output = ()>`.

Translation: an object that implements the future trait with the associated type `Object` being the unit type.

We could also expose our custom future and return `Hello` directly.

(this works the same, because Hello implements the `Future` trait)

What about the body of the function?

We construct the `Init` variant of our enum and return it.

Now it's starting to become clear why an async function doesn't do anything if you don't await it.

We're not doing anything!

Just constructing an object, nothing else gets run.

So let's call our future.

We can't call `poll()`.

We don't have a `Context` to pass to it.

(we could create a `Context`, but that's a story for another day)

(remember we want to understand how async/await works for the user, not for the async runtime)

Luckily, the `await` keyword works just fine on "hand made" futures.

(this is what the compiler creates under the hood, after all)

So let's await our future!

Here's our main function.

(note that it **is** async, we must always be in `async` context to await a future)

```rust
#[tokio::main]
async fn main() {
    hello("world").await;
}
```

Well, that's boring.

It's **exactly** the same as when `hello()` was an async function.

What about the output?

```text
hello, world!
```

Also exactly the same.

This might seem like a bit of an anti-climax.

But remember, you've just written your first custom future in Rust!

(or maybe your second future, or your hundredth)

(have you really written 100 futures? nice!)

#### future sequence diagram

Here's a sequence diagram of our `Hello` future.

![Sequence diagram of our hello world async function.](/img/understanding-async-await-1/hello.svg)

I'll be creating similar sequence diagrams for each of the futures we write in this series.

Hopefully that will help tie the different concepts together.

