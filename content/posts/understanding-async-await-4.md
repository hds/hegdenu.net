+++
title = "how I finally understood async/await in Rust (part 4)"
slug = "understanding-async-await-4"
author = "hds"
date = "2023-08-31"
draft = true
+++

You've reached the end of my series on understanding async/await in Rust.

(the beginning of the end)

Throughout the last posts we've asked a series of questions.

The answers to those questions helped us understand how async/await works.

We did this by looking at how to "manually" implement futures.

(this is what the `async`/`.await` syntax hides from us)

But that syntax is really just sugar on top of implementations of the `Future` trait.

Everyone likes sugar, so why would we ever **not** want to use async/await?

(I know that not everyone likes sugar)

(this is where my analogy really falls apart)

This is our final question.

These are the questions we've asked so far.

* [part 1: why doesn’t my task do anything if I don’t await it?](@/posts/understanding-async-await-1.md)
* [part 2: how does a pending future get woken?](@/posts/understanding-async-await-2.md)
* [part 3: why shouldn’t I hold a mutex guard across an await point?](@/posts/understanding-async-await-3.md)
* [part 4: why would I ever want to write a future manually?](#why-would-i-ever-want-to-write-a-future-manually) (this is it)

And why would I?

Let's have a look.

## why would I ever want to write a future manually?

Rust's async/await syntax allows us to easily write async code.

But this isn't the best bit.

This syntax allows us to write async code that is easy to **read**.

It's much easier to follow code written with async/await than the alternatives.

Let's say callbacks.

(like Javascript)

Or something even uglier like delegates from Objective-C.

(I'm going to leave [this here for the curious](https://developer.apple.com/documentation/foundation/nsurlconnectiondelegate?language=objc))

But implementing the `Future` trait in Rust makes those patterns look welcoming!

(enough complaining now)

In reality, async/await allows you to compose futures.

These may be other async functions or blocks.

(which are just futures underneath)

These may be futures written by clever people.

(like the ones who write async runtimes)

But some things can't be done without the implementation details.

Specifically, for some things, you need access to that [waker](@/posts/understanding-async-await-2.md#the-waker).

## why we need a waker

Remember that when a future returns `Poll::Pending`, that will generally be propagated up to the task.

The future driving the task will return `Poll::Pending`.

Then the task won't be polled again immediately.

In fact, it won't be polled again at all until it gets woken.

In [part 2](@/posts/understanding-async-await-2.md), we built the [`YieldNow`](@/posts/understanding-async-await-2.md#yield-now) future.

It returns `Poll::Pending`, but not before waking the task.

This causes the runtime to schedule the task to be polled again as soon as possible.

This isn't a very interesting future.

(although we learned a lot from it)

An interesting future wouldn't wake the task **before** returning `Poll::Pending`.

It would wake the task at some point later when something is ready that wasn't ready immediately.

So let's build an interesting future.

### an interesting future

(this is the band formed after [pending future](@/posts/understanding-async-await-2.md#pending-future) split up)

A really interesting future would deal with networking or other OS level things.

But that's beyond the level of this series of posts.

(this is a bit of a lie)

(the truth is it's beyond me)

So let's build something that isn't too complex.

And ideally has no large dependencies.

(either knowledge or code wise)

We're going to build an async channel!

## aside: channels

Channels are a form of message passing.

They are not a concept exclusive to async code or Rust.

(neither is message passing obviously)

Most channels can be thought of as a queue.

Messages go in one end and come out the other.

Channels can be classified by where the messages come from, where they go, and how they're replicated.

How many producers can put messages into the channel?

How many consumers can take messages out of the channel?

How many times does each message get delivered?

The Tokio docs have a nice description of their [message passing](https://docs.rs/tokio/latest/tokio/sync/index.html#message-passing) options.

Let's quickly run through some of those channels.

We can visualize each of them.

This will help us understand what's missing.

### oneshot

A oneshot channel support sending a single value from a single producer to a single consumer.

Let's look at a sequence diagram.

![Sequence diagram of a oneshot channel. A single messages goes from the producer to the channel and then to the consumer.](/img/understanding-async-await-4/channel_oneshot-sequence_diagram.svg)

This diagram isn't particularly interesting.

But it forms the basis for the next ones.

Important parts are that there is only one of each thing.

A single producer.

A single consumer.

And a single message.

(actually Tokio's `oneshot` channels can be reused, but that's not important here)

Here's the reference to the one in Tokio: [`tokio::sync::oneshot`](https://docs.rs/tokio/latest/tokio/sync/oneshot/index.html).

### multi-producer single-consumer (mpsc)

That's what MPSC stands for!

(this acronym is often used without any description)

(now you know what it means)

These channels have multiple producers.

But just a single consumer.

And many messages can be sent across them.

Quick aside-aside.

Often we distinguish bounded from unbounded channels.

Bounded channels have a fixed size buffer.

Unbounded channels have an unlimited buffer.

(so they will keep filling up until the system runs out of memory)

We will gloss over this when discussing channels on a high level.

Let's look at how it works.

![Sequence diagram of a multi-producer single-consumer (mpsc) channel. Two producers send messages to the channel, the single consumer receives those messages in the order they were received by the channel.](/img/understanding-async-await-4/channel_mpsc-sequence_diagram.svg)

First big difference is that there are multiple producers.

(no big surprises there)

They each some some messages to the channel.

The channel then delivers those messages to the consumer.

They are delivered in order.

(we usually say they are delivered in the order they were sent)

(it's really the order in which the channel received the messages)

(this is **mostly** the same thing)

The one in Tokio is [`tokio::sync::mpsc`](https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html).

### broadcast

After multi-producer single consumer channels comes...

Multi-producer multi-consumer (MPMC) channels!

(sort of)

A broadcast channel is always a multi-consumer channel.

But with something special.

All consumers receive all values.

Some broadcast channels may only allow a single producer (SPMC).

(single-producer multi-consumer)

Others allow multiple producers (MPMC).

(multi-producer multi-consumer)

But normally we reserve MPMC for a different type of channel.

For now, let's look at the sequence diagram.

![Sequence diagram of a broadcast channel. One producer sends messages to the channel, the two consumer receive each of those messages in the order they were received by the channel.](/img/understanding-async-await-4/channel_broadcast-sequence_diagram.svg)

Both of the receivers get the same sequence of messages.

That's the key here.

Tokio also has one: [`tokio::sync::broadcast`](https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html).

### multi-producer multi-consumer (mpmc)

Another acronym that we now understand!

Multi-producer multi-consumer.

As we just saw, a broadcast channel can be a sort of MPMC channel.

The difference is that each message will only be received by a single consumer.

This is the sort of channel you would use to distribute work a finite number of tasks.

The sequence diagram shows the difference.

![Sequence diagram of a mpmc channel. Two producers send messages to the channel, each message is received by only a single consumer. The messages are received by the consumers in the order that they were received by the channel.](/img/understanding-async-await-4/channel_mpmc-sequence_diagram.svg)

Like all the channels we've seen, this channel is ordered.

The consumers receive messages in the same order they were sent.

But there is no concept of fairness amongst consumers.

The first consumer to try to receive a message gets the next message.

(we could also say that there is no load balancing)

Now, where's that link to the implementation in Tokio?

There isn't one!

Tokio doesn't have an implementation of an MPMC channel.

So let's build one!

But first, some truth telling.

## aside: async-channel

There are implementations of MPMC channels available.

The [`async-channel`](https://crates.io/crates/async-channel) crate provides one.

It's part of the [`smol-rs`](https://github.com/smol-rs) project.

Smol is another async runtime for Rust.

It's more modular than Tokio.

As a result, some parts of it can be dropped into a Tokio runtime and just work.

But we can still learn something from building our own.

So let's do that!

## our own mpmc channel

We're going to write our own multi-producer multi-consumer channel.

And it's going to be simple.

(you know I'm all about simple)

Let's begin with the API.

We'll base it on the `std` library and Tokio channel APIs.

### channel API

Here's the signature for our `channel()` function.

(with rustdocs, you always include rustdocs, rigth?)

```rust
/// Creates a new asynchronous bounded multi-producer multi-consumer channel,
/// returning the sender/receiver halves.
///
/// The channel will buffer messages up to the defined capacity. Once the
/// buffer is full, attempts to send new messages will wait until a message is
/// received from the channel. When the channel is empty, attempts to receive
/// new messages will wait until  a message is sent to the channel.
///
/// If all receivers or all senders have disconnected, the channel will be
/// closed. Subsequent attempts to send a message will return a
/// [`ChannelClosedError`]. Subsequent attempts to receive a message will drain
/// the channel and once it is empty, will also return a [`ChannelClosedError`].
pub fn channel(capacity: usize) -> (Sender, Receiver)
```

OK, so we have much more docs than function signature.

Let's break it down.

The function returns sender and receiver halves.

This is standard Rust practice for constructing a channel.

The two halves can be passed around as needed.

Independent of one another.

(we'll look at the halves shortly)

In the second paragraph, we specify that our channel will buffer messages.

So the function requires the capacity.

So `capacity` messages can be sent without any being received.

Then what?

Then the channel is full.

And the senders will wait until some message is received.

(wait asynchronously of course)

On the other side, the channel might be empty.

Then the receivers will wait until some message is sent.

(yes, yes, wait asynchronously)

Finally we have a bit about closing the channel.

There are two ways the channel could get closed.

If all the receivers disconnect from the channel.

(this means all the receivers are dropped)

This means that no messages will be received.

So there's no point in sending any more.

(queue the song Unsent Letter by MGF)

So we'll alert the senders by returning an error next time a send is attempted.

The other way is if all the senders disconnect from the channel.

In this case, no new messages will be sent.

But there may be messages in the channel already.

So we'll let the receivers drain the channel.

(retrieving any messages in there)

But once the channel is empty, new receive attempts would block forever.

(I don't have a song for this one)

(Really have used Unsent Letter here instead)

Instead, an empty and closed channel will return an error to receive.

That seems pretty clear.

Let's look at the sender and receiver halves.

One more thing.

This channel only sends `String`s.

(I know this is boring)

(but let's focus on async stuff, not generics stuff)

### channel halves

(I really wanted to call this section "you complete me")

First let's look at the sender.

```rust
/// The sending-half of the [`mpmc::channel`] type.
///
/// Messages can be sent through the channel with [`send`].
///
/// This half can be cloned to send from multiple tasks. Dropping all senders
/// will cause the channel to be closed.
///
/// [`mpmc::channel`]: fn@super::mpmc::channel
/// [`send`]: fn@Self::send
pub struct Sender
```

Nothing much new on the struct itself.

We can clone it.

(but we don't derive `Clone`, you'll see why later)

It can be used to send messages.

Let's have a look at that method.

```rust
impl Sender {
    /// Sends a value, waiting until there is capacity.
    ///
    /// A successful send occurs when there is at least one [`Receiver`] still
    /// connected to the channel. An `Err` result means that the value will
    /// never be received, however an `Ok` result doesn't guarantee that the
    /// value will be received as all receivers may disconnect immediately
    /// after this method returns `Ok`.
    pub async fn send(&self, value: String) -> Result<(), ChannelClosedError>
}
```

Remember, we're just sending strings today.

(making this channel generic is left as an exercise for the reader)

Our public API is an async function that takes the value to be sent.

It returns a result.

The result can either be `Ok` with the unit type `()`.

Or it could be our error.

There is only one possible error, that the channel is closed.

Now let's look at the receiver.

```rust
/// The receiving-half of the [`mpmc::channel`] type.
///
/// Messages can be received from the channel with [`recv`].
///
/// This half can be cloned to receive from multiple tasks. Each message will
/// only be received by a single receiver. Dropping all receivers will cause
/// the channel to be closed.
///
/// [`mpmc::channel`]: fn@super::mpmc::channel
/// [`recv`]: fn@Self::recv
pub struct Receiver
```

Once again, what we expect.

And again, we claim to be able to clone the receiver.

But we don't derive `Clone`.

We also only implement a single public (async) function on `Receiver`.

```rust
impl Receiver {
    /// Receives a value, waiting until one is available.
    ///
    /// Once the channel is closed (by dropping all senders), this method will
    /// continue to return the remaining values stored in the channel buffer.
    /// Once the channel is empty, this method will return
    /// [`ChannelClosedError`].
    pub async fn recv(&self) -> Result<String, ChannelClosedError>
}

```

Upon success, `recv` will return a `String`.

The only error it can return is `ChannelClosedError`.

And this is only returned if the channel is empty.

Now we know the API we'd like to implement.

Let's look at an example sequence diagram of how it would be used.

We'll just use a single produce and single consumer to understand the async/await part better

![Sequence diagram of an mpmc channel. A main task creates a channel and then sends the receiver to a task to loop over receiving. the sender is sent to a different task to send 2 values.](/img/understanding-async-await-4/mpmc_api-sequence_diagram.svg)

Our main task calls `channel(1)` and gets the sender and receiver back.

This is a channel with a capacity of one.

(not very big)

The receiver is sent to its own task to receive in a loop.

We now imagine that it tries once.

But the call to `recv().await` has to wait because the channel is empty.

Now the sender gets sent to its own task to send two values.

The first is sent.

Then it attempts to send the second value.

(we're picking our own concurrent interleaving here)

(it makes the story more interesting)

The second value can't be sent as the channel is full.

So the call to `send().await` waits.

Our receiver now receives a value.

As the channel now has capacity, the waiting call to `send().await` returns.

The sending task now completes.

This leaves our receiver.

It receives the second value.

(all good)

Then it tries to receive again.

This time it gets an `Err(ChannelClosedError)` back.

Since the sender was dropped, the channel is closed.

And now it's empty as well.

So our receiving loop task also completes.

With this, we have a basis for understanding how our API should work.

Time to look at implementing this async functionality.

This will involve implementing these two async functions.

`Sender::send` and `Receiver::recv`.

To do this we will need to implement futures manually.

(that is the whole point of this series in a way)

(understanding async/await by implementing the bits under it)

So let's look at each of these async functions and the futures underneath them in turn.

### inner channel

One thing, before we start implementing futures.

The senders and receivers will all share the channel.

We're going to do that the easy way.

A private struct wrapped in `Arc` and `Mutex`.

We learnt about [mutexes](@/posts/understanding-async-await-3.md#aside-mutexes-and-mutex-guards-in-rust) in part 3.

(we also learnt about some things we shouldn't do with them)

(so we won't do that stuff)

Using a `Mutex` in this way isn't generally very efficient.

It will definitely not perform well under high load.

But it will serve to let us write our send and receive futures.

And we'll know that we won't cause any data corruption or data races.

So we'll create a `Channel` struct.

This won't be public.

### send future

We're going to start with implementing `Sender::send`.

We'll look at the contents of our internal channel later.

Let's look at a sequence diagram of our three different outcomes.

As we prepare to implement each layer, we'll expand upon this diagram.

![Sequence diagram of the use of the async function Sender::send. It covers three use cases: channel has capacity, channel is closed, and channel is full.](/img/understanding-async-await-4/mpmc_send_async-sequence_diagram.svg)

This diagram is on an async/await level.

The three outcomes represent three states that the channel can be in.

**State: channel has capacity.**

In this case, the async function returns `Ok` straight away.

(an async function returning straight away isn't the same as any other function returning straight away)

(it could still yield to the runtime in the meantime)

**State: channel is closed.**

This is a terminal state for the inner channel.

(that means it's the last state, there are no more states after it)

The async function will immediately return the channel closed error.

(same caveat on using "immediately" with "async")

**State: channel is full.**

In this case, the async function will wait, not return anything.

(we know there is probably a `Poll::Pending` needed here)

(but let's not get ahead of ourselves)

At some point, capacity will be freed

(we hope)

This would happen when a receiver receives a message.

Then the async function will return `Ok` as in the first case.

(since we've essentially moved to the first state)

Let's go on to implement our async function.

We won't go into the details of constructing the `Sender` struct.

For now, it's enough to know that `Sender` has a private field giving it access to the inner channel.

```rust
pub struct Sender {
    inner: Arc<Mutex<Channel>>,
}
```

To implement that public async function, we're going to need a `Future`.

We'll call our future `Send`.

(not very imaginative, which is probably a good thing)

(imaginative names make less sense to others)

First, here's that implementation of `Sender::send`.

```rust
pub async fn send(&self, value: String) -> Result<(), ChannelClosedError> {
    Send {
        value,
        inner: self.inner.clone(),
    }
    .await
}
```

Pretty simple, right?

We construct our `Send` future.

And then await it.

To be clear about the types, here is the struct definition for `Send`.

```rust
struct Send {
    value: String,
    inner: Arc<Mutex<Channel>>,
}
```

Our `Send` future has just one job

Send a value when the channel has capacity.

We don't even need state for this.

We will instead depend on the state of the channel.

(and we'll trust our async runtime to not poll us after we've returned `Poll::Ready`)

We do need the value itself.

And the `Send` future may outlive the `Sender`.

(although this isn't ideal)

So we'll take our own reference to the inner channel.

Before we look at the implementation, let's expand our sequence diagram.

We're diving through the async/await syntax and into future land.

![Sequence diagram of the use of the future Send. It covers three use cases: channel has capacity, channel is closed, and channel is full.](/img/understanding-async-await-4/mpmc_send_future-sequence_diagram.svg)

As you can see, we've already created part of the API for our inner channel.

This is based purely on what we know we need.

It has a (synchronous) `send()` function which returns a result.

Either `Ok(())` or one of 2 errors depending on whether the channel is closed or full.

The first two states are fairly straight forward.

(channel has capacity and channel is closed)

So let's look at channel is full.

In this case, `Channel::send` will return an error stating that the channel is full.

Our `Send` future will return `Poll::Pending`.

But first...

We need a way to register our waker with the channel.

Then we'll expect to get woken when there is free capacity.

For this, the channel has another method, `Channel::register_sender_waker()`.

The diagram cheats a little bit.

We know that the channel won't [wake our task directly](@/posts/understanding-async-await-2.md#yield-now).

We'll also skip over the channel implementation.

It's enough that we have a requirement.

When we register a sender waker, the channel must wake it when there is capacity.

Of course, there may be multiple senders and they may have all registered wakers.

So we can't expect to be woken for the next free capacity.

But that's an inner channel implementation detail.

Now let's dive into the `Future` implementation.

```rust
impl Future for Send {
    type Output = Result<(), ChannelClosedError>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let Ok(mut guard) = self.inner.lock() else {
            panic!("MPMC Channel has become corrupted.");
        };

        match guard.send(self.value.clone()) {
            Ok(_) => Poll::Ready(Ok(())),
            Err(ChannelSendError::Closed) => Poll::Ready(Err(ChannelClosedError {})),
            Err(ChannelSendError::Full) => {
                guard.register_sender_waker(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}
```

The `Output` will be the same value that `Sender::send` returns.

To begin with, we'll try to lock the mutex wrapping our inner channel.

If this returns an error, the inner channel is corrupted.

(actually, the mutex is poisoned)

(we looked a little bit at poisoning in [how rust does mutexes](@/posts/understanding-async-await-3.md#how-rust-does-mutexes) from part 3)

In any event, we'll panic and be done with it.

Then we'll call `send()` on our inner channel.

We've already gone over teh details of what happens here in the sequence diagram.

One implementation detail is that we clone the value to send to the inner channel.

This could be avoided.

But the implementation would be much more complex.

(it's because if the channel is full, we need our value back again)

So we'll leave it like this for now.

Tokio's channels use semaphore permits to correctly implement this without cloning.

That's the implementation of our `Send` future!

In the end it wasn't so scary.

We already knew what we needed to do.

Now we can look at the receive future.

Later we'll go over the implementation of the inner channel.

(that's mostly for completeness sake)

(and also because we want to cover actually using that registered waker)
