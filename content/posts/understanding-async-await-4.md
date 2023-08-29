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

But we can still learn something from builder our own.

So let's do that!


