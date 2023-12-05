+++
title = "debugging tokio instrumentation"
slug = "debugging-tokio-instrumentation"
author = "hds"
date = "2023-12-31"
draft = true
+++

I contribute to [`tokio-console`](https://github.com/tokio-rs/console). One of the things that I
often find myself doing, is matching what is shown in the console with the "raw"
[`tracing`](https://docs.rs/tracing) output that comes from Tokio. However, this is pretty hard to
read and doesn't actually contain all the information that I need.

There are a couple of things that I'd like to have. Firstly (and most importantly), I need to see
Tracing's internal span ID for the spans that are output. This is something which the
[`fmt::Subscriber`] (and underlying Layer) don't support. And probably rightly so - it's internal
information. But it's used heavily in the instrumentation in Tokio and to debug, I really need
to have it available.

[`fmt::Subscriber`]: https://docs.rs/tracing-subscriber/0.3.18/tracing_subscriber/fmt/struct.Subscriber.html

Normally, to get this information I use a patched version of the `tracing-subscriber` crate. But
this is something that can't be checked into the console project, and setting it up each time is a
bit tedious.

Secondly, I'd like to be able to visually differentiate different types of spans which are specific
to Tokio's instrumentation. Unlike the internal span ID, this is entirely domain specific, and has
no use outside of this specific use case.

Having now justified something I wanted to do anyway, let's build our own custom tracing subscriber!

(actually, it's going to mostly be a `Layer`)

### aside: tracing subscribers and layers

In the tracing ecosystem, you need a subscriber to actually do anything other than send your traces
into the void. Specifically something that implements the [`Subscriber`] trait. A subscriber can
take the traces and do what it wishes. Write them to `stdout`, to a file, collect them and perform
aggregation, send them to another service (maybe via Open Telemetry).

[`Subscriber`]: https://docs.rs/tracing/latest/tracing/trait.Subscriber.html

The [`tracing-subscriber`] crate provides a number of subscriber implementations. From the outside,
this mostly looks like different ways to write traces to a file handle. However, the real heart of
[`tracing-subscriber`] is the [registry]. The registry is subscriber which implements a span store
and allows multiple layers to connect to it.

[`tracing-subscriber`]: https://docs.rs/tracing-subscriber
[registry]: https://docs.rs/tracing-subscriber/0.3.18/tracing_subscriber/registry/index.html

What is a [`Layer`]? For the longest time I had real trouble understanding conceptually what a
layer is. From the documentation, a layer is *"composable abstraction for building Subscribers"*.
However, I struggled to understand how I may wish to compose layers. It's also confusing because
layers don't feed into other layers the way that [`tower`] layers do (which are like middleware,
in that what one layer does affects what the next layer receives).

[`Layer`]: https://docs.rs/tracing-subscriber/0.3.18/tracing_subscriber/layer/trait.Layer.html
[`tower`]: https://docs.rs/tower

Instead, think of layers as mini-subscribers. They can take action on some methods on the [`Layer`]
trait, but can fall back on the default implementation for things that they're not interested in.
And [`Layer`] has a default implementation for everything.

Most layers need to store information about spans... TDB