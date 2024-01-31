+++
title = "debugging tokio instrumentation"
slug = "debugging-tokio-instrumentation"
author = "hds"
date = "2024-01-30"
+++

I contribute to [`tokio-console`](https://github.com/tokio-rs/console). One of the things that I
often find myself doing is matching what is shown in the console with the "raw" [`tracing`] output
that comes from Tokio. However, this is pretty hard to read and doesn't actually contain all the
information that I need.

[`tracing`]: https://docs.rs/tracing

There are a couple of things that I'd like to have. Firstly (and most importantly), I need to see
Tracing's internal [`span::Id`] for the spans that are emitted. This is something which the
[`fmt::Subscriber`] (and underlying Layer) don't support. And rightly so - it's internal
information. But it's used heavily in the instrumentation in Tokio and to debug, I really need
to have it available.

[`span::Id`]: https://docs.rs/tracing/0.1.40/tracing/span/struct.Id.html
[`fmt::Subscriber`]: https://docs.rs/tracing-subscriber/0.3.18/tracing_subscriber/fmt/struct.Subscriber.html

Normally, to get this information I use a patched version of the `tracing-subscriber` crate. But
this is something that can't be checked into the console project, and setting it up each time is a
bit tedious.

Secondly, I'd like to be able to visually differentiate the specific spans and events used in
Tokio's instrumentation. Unlike the internal span ID, this is entirely domain specific, and has
no use outside of this specific use case.

Here's a snippet of output from the [`fmt::Subscriber`] outputting some of Tokio's instrumentation.

<pre data-lang="custom" style="background-color:#2b303b;color:#c0c5ce;" class="language-custom "><code class="language-custom" data-lang="custom"><span style='opacity:0.67'>2024-01-31T09:23:11.247690Z</span> <span style='color:#0a0'> INFO</span> <b>runtime.spawn{</b><i>kind</i><span style='opacity:0.67'>=</span>task <i>task.name</i><span style='opacity:0.67'>=</span> <i>task.id</i><span style='opacity:0.67'>=</span>18 <i>loc.file</i><span style='opacity:0.67'>=</span>&quot;src/bin/initial-example.rs&quot; <i>loc.line</i><span style='opacity:0.67'>=</span>11 <i>loc.col</i><span style='opacity:0.67'>=</span>5<b>}</b><span style='opacity:0.67'>:</span> <span style='opacity:0.67'>initial_example:</span> pre-yield <i>fun</i><span style='opacity:0.67'>=</span>true
<span style='opacity:0.67'>2024-01-31T09:23:11.247703Z</span> <span style='color:#a0a'>TRACE</span> <b>runtime.spawn{</b><i>kind</i><span style='opacity:0.67'>=</span>task <i>task.name</i><span style='opacity:0.67'>=</span> <i>task.id</i><span style='opacity:0.67'>=</span>18 <i>loc.file</i><span style='opacity:0.67'>=</span>&quot;src/bin/initial-example.rs&quot; <i>loc.line</i><span style='opacity:0.67'>=</span>11 <i>loc.col</i><span style='opacity:0.67'>=</span>5<b>}</b><span style='opacity:0.67'>:</span> <span style='opacity:0.67'>tokio::task::waker:</span> <i>op</i><span style='opacity:0.67'>=</span>&quot;waker.clone&quot; <i>task.id</i><span style='opacity:0.67'>=</span>2
<span style='opacity:0.67'>2024-01-31T09:23:11.247717Z</span> <span style='color:#a0a'>TRACE</span> <b>runtime.spawn{</b><i>kind</i><span style='opacity:0.67'>=</span>task <i>task.name</i><span style='opacity:0.67'>=</span> <i>task.id</i><span style='opacity:0.67'>=</span>18 <i>loc.file</i><span style='opacity:0.67'>=</span>&quot;src/bin/initial-example.rs&quot; <i>loc.line</i><span style='opacity:0.67'>=</span>11 <i>loc.col</i><span style='opacity:0.67'>=</span>5<b>}</b><span style='opacity:0.67'>:</span> <span style='opacity:0.67'>tokio::task:</span> exit
<span style='opacity:0.67'>2024-01-31T09:23:11.247737Z</span> <span style='color:#a0a'>TRACE</span> <span style='opacity:0.67'>tokio::task::waker:</span> <i>op</i><span style='opacity:0.67'>=</span>&quot;waker.wake&quot; <i>task.id</i><span style='opacity:0.67'>=</span>2
<span style='opacity:0.67'>2024-01-31T09:23:11.247754Z</span> <span style='color:#a0a'>TRACE</span> <b>runtime.spawn{</b><i>kind</i><span style='opacity:0.67'>=</span>task <i>task.name</i><span style='opacity:0.67'>=</span> <i>task.id</i><span style='opacity:0.67'>=</span>18 <i>loc.file</i><span style='opacity:0.67'>=</span>&quot;src/bin/initial-example.rs&quot; <i>loc.line</i><span style='opacity:0.67'>=</span>11 <i>loc.col</i><span style='opacity:0.67'>=</span>5<b>}</b><span style='opacity:0.67'>:</span> <span style='opacity:0.67'>tokio::task:</span> enter
<span style='opacity:0.67'>2024-01-31T09:23:11.247766Z</span> <span style='color:#a0a'>TRACE</span> <b>runtime.resource{</b><i>concrete_type</i><span style='opacity:0.67'>=</span>&quot;Barrier&quot; <i>kind</i><span style='opacity:0.67'>=</span>&quot;Sync&quot; <i>loc.file</i><span style='opacity:0.67'>=</span>&quot;src/bin/initial-example.rs&quot; <i>loc.line</i><span style='opacity:0.67'>=</span>9 <i>loc.col</i><span style='opacity:0.67'>=</span>39<b>}</b><span style='opacity:0.67'>:</span> <span style='opacity:0.67'>tokio::sync::barrier:</span> enter
<span style='opacity:0.67'>2024-01-31T09:23:11.247800Z</span> <span style='color:#a0a'>TRACE</span> <b>runtime.resource{</b><i>concrete_type</i><span style='opacity:0.67'>=</span>&quot;Barrier&quot; <i>kind</i><span style='opacity:0.67'>=</span>&quot;Sync&quot; <i>loc.file</i><span style='opacity:0.67'>=</span>&quot;src/bin/initial-example.rs&quot; <i>loc.line</i><span style='opacity:0.67'>=</span>9 <i>loc.col</i><span style='opacity:0.67'>=</span>39<b>}</b><span style='opacity:0.67'>:</span><b>runtime.resource.async_op{</b><i>source</i><span style='opacity:0.67'>=</span>&quot;Barrier::wait&quot; <i>inherits_child_attrs</i><span style='opacity:0.67'>=</span>false<b>}</b><span style='opacity:0.67'>:</span> <span style='opacity:0.67'>tokio::util::trace:</span> new
</code></pre>

There is a lot of information here, and distinguishing different spans types can be complicated
(especially when you're scanning through dozens or even hundreds of lines). Additionally, the
[`span::Id`] is completely absent.

Compare this to the output of the same section of logs coloured and including the [`span::Id`]
right after the span name.

<pre data-lang="custom" style="background-color:#2b303b;color:#c0c5ce;" class="language-custom "><code class="language-custom" data-lang="custom"><span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>09:23:39</span></b></span><span style='opacity:0.67'>.136879Z</span> <span style='color:#489e6c'> INFO</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>2</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=, task.id=18, loc.file=&quot;src/bin/initial-example.rs&quot;, loc.line=18, loc.col=5}</span> <b><span style='color:#aaa'>initial_example</span></b>: <span style='color:#aaa'>fun=true pre-yield</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>09:23:39</span></b></span><span style='opacity:0.67'>.136937Z</span> <span style='color:#9d4edd'>TRACE</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>2</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=, task.id=18, loc.file=&quot;src/bin/initial-example.rs&quot;, loc.line=18, loc.col=5}</span> <b><span style='color:#c77dff'>tokio::task::waker</span></b>: <span style='color:#9d4edd'>op=&quot;waker.clone&quot;, task.id=2</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>09:23:39</span></b></span><span style='opacity:0.67'>.136995Z</span> <span style='color:#9d4edd'>TRACE</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>2</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=, task.id=18, loc.file=&quot;src/bin/initial-example.rs&quot;, loc.line=18, loc.col=5}</span> <b><u><span style='color:#5aba84'>exit</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>09:23:39</span></b></span><span style='opacity:0.67'>.137059Z</span> <span style='color:#9d4edd'>TRACE</span> <b><span style='color:#c77dff'>tokio::task::waker</span></b>: <span style='color:#9d4edd'>op=&quot;waker.wake&quot;, task.id=2</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>09:23:39</span></b></span><span style='opacity:0.67'>.137122Z</span> <span style='color:#9d4edd'>TRACE</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>2</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=, task.id=18, loc.file=&quot;src/bin/initial-example.rs&quot;, loc.line=18, loc.col=5}</span> <b><u><span style='color:#5aba84'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>09:23:39</span></b></span><span style='opacity:0.67'>.137212Z</span> <span style='color:#9d4edd'>TRACE</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>1</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Barrier&quot;, kind=&quot;Sync&quot;, loc.file=&quot;src/bin/initial-example.rs&quot;, loc.line=16, loc.col=39}</span> <b><u><span style='color:#df5853'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>09:23:39</span></b></span><span style='opacity:0.67'>.137296Z</span> <span style='color:#9d4edd'>TRACE</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>1</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Barrier&quot;, kind=&quot;Sync&quot;, loc.file=&quot;src/bin/initial-example.rs&quot;, loc.line=16, loc.col=39}</span> <span style='color:#5c8dce'>runtime.resource.async_op[<b><span style='color:#508ee3'>274877906945</span></b></span><span style='color:#5c8dce'>]{source=&quot;Barrier::wait&quot;, inherits_child_attrs=false}</span> <b><u><span style='color:#508ee3'>new</span></u></b>
</code></pre>

Having now justified something I wanted to do anyway, let's build our own custom tracing subscriber!

(actually, it's going to mostly be a `Layer`)

## aside: tracing subscribers and layers

If you're already familiar with `tracing`, you may wish to skip this section and go straight to
[ari-subscriber](#ari-subscriber).

In the tracing ecosystem, you need a subscriber to actually do anything other than send your traces
into the void. Specifically something that implements the [`Subscriber`] trait. A subscriber can
take the traces and do what it wishes. Write them to `stdout`, to a file, collect them and perform
aggregation, send them to another service (maybe via Open Telemetry).

[`Subscriber`]: https://docs.rs/tracing/latest/tracing/trait.Subscriber.html

The [`tracing-subscriber`] crate provides a number of subscriber implementations. From the outside,
this mostly looks like different ways to write traces to a file handle. However, the real heart of
[`tracing-subscriber`] is the [registry]. The registry is a subscriber which implements a span
store and allows multiple layers to connect to it.

[`tracing-subscriber`]: https://docs.rs/tracing-subscriber
[registry]: https://docs.rs/tracing-subscriber/0.3.18/tracing_subscriber/registry/index.html

What is a [`Layer`]? For the longest time I had real trouble understanding conceptually what a
layer is. From the documentation, a layer is *"a composable abstraction for building Subscribers"*.
However, I struggled to understand how I may wish to compose layers. It's also confusing because
layers don't feed into other layers the way that [`tower`] layers do (which are like middleware,
in that what one layer does affects what the next layer receives).

[`Layer`]: https://docs.rs/tracing-subscriber/0.3.18/tracing_subscriber/layer/trait.Layer.html
[`tower`]: https://docs.rs/tower

Instead, think of layers as mini-subscribers. They can take action on some methods on the [`Layer`]
trait, but can fall back to the default implementation for things that they're not interested in.
And [`Layer`] has a default implementation for everything.

Most layers need to store information about spans, this is where the [registry] comes in
(specifically via the [`LookupSpan`] trait). Layers can store their own data in the registry in the
form of span [extensions].

[`LookupSpan`]: https://docs.rs/tracing-subscriber/0.3.18/tracing_subscriber/registry/trait.LookupSpan.html
[extensions]: https://docs.rs/tracing-subscriber/0.3.18/tracing_subscriber/registry/trait.SpanData.html#tymethod.extensions

The reason why storing this data in the registry is important may not be immediately obvious.

It's because [`tracing`] itself **doesn't** store this data. This allows [`tracing`] to not
allocate for the data and therefore be used in [`no_std`] environments as well as the giant servers
and beefy development machines that many of us are accustomed to.

[`no_std`]: https://docs.rust-embedded.org/book/intro/no-std.html

Here's an example for clarity. When a span is created, a [`Subscriber`] receives a call to
[`new_span()`]. This includes the span [`Attributes`] which gives the subscriber access to all the
information about that span. Its metadata, field names, and also the values of any fields that were
set at the time of creation.

[`new_span()`]: https://docs.rs/tracing/0.1.40/tracing/trait.Subscriber.html#tymethod.new_span
[`Attributes`]: https://docs.rs/tracing/0.1.40/tracing/span/struct.Attributes.html

This is great, it's everything we could need!

Now let's look at the method that gets called when a span is entered (becomes active), this is
called [`enter()`] and all it comes with is... a [`span::Id`]. No metadata, no field names, and
certainly no field values. And this pattern repeats on the trait methods called when a span exits
or is closed.

[`Id`]: https://docs.rs/tracing/0.1.40/tracing/span/struct.Id.html

Using the registry to store whatever data a layer might need about a span later on is the solution.
This allows the [`fmt::Subscriber`] to print out the full data for each span in an event's
ancestry.

Now that we understand a bit about what subscribers and layers are, let’s get into implementing
some of it!

## ari-subscriber

To meet the needs of my use-case, as described above, I've written the [`ari-subscriber`] crate.
It's currently at version 0.0.1, which indicates that it's probably a bit rough, but so far it's
already helped me quickly narrow down the version of Tokio after which `yield_now()` [doesn't get
detected as a self wake by Tokio Console](https://github.com/tokio-rs/console/issues/512).

[`ari-subscriber`]: https://docs.rs/ari-subscriber/0.0.1/ari_subscriber/index.html

The “ari” in ari-subscriber is for “async runtime instrumentation”.


The interface is simple, you pass an `ari-subscriber` layer to the [registry]:

```rust
use tracing_subscriber::prelude::*;
tracing_subscriber::registry()
    .with(ari_subscriber::layer())
    .init();
```

This will write output to `stdout` (currently not configurable). And the output will have pretty
colours!

Let's look at a simple example of how we can use `ari-subscriber`. Here's the Rust code we'll be
using:

```rust
#[tokio::main]
async fn main() {
    // Set up subscriber
    use tracing_subscriber::prelude::*;
    tracing_subscriber::registry()
        .with(ari_subscriber::layer())
        .init();

    // Spawn a task and wait for it to complete
    tokio::spawn(async {
        tracing::info!(fun = true, "pre-yield");
        tokio::task::yield_now().await;
    })
    .await
    .unwrap();
}
```

We start in an async context (using the `#[tokio::main]` attribute). First we set up the
`ari-subscriber` layer with the registry. Then we spawn a task and wait for it to complete. The
task emits a tracing event and then returns control to the runtime by calling the [`yield_now()`]
function from Tokio. After that it ends

[`yield_now()`]: https://docs.rs/tokio/1.35.1/tokio/task/fn.yield_now.html

If you've been watching closely (and following all the links I've been sprinkling around), you may
have realised that I'm looking at the case described in the issue [console#512]. What we want to
look at is where the wake operation occurs.

[console#512]: https://github.com/tokio-rs/console/issues/512

We're going to fix our version of Tokio to an old one, where we know that Tokio Console detects
awaiting on [`yield_now()`] as a self-wake. So let's specify the following in our `Cargo.toml`:

```toml
[dependencies]
ari-subscriber = "0.0.1"
tokio = { version = "=1.22.0", features = ["full", "tracing"] }
tracing = "0.1.40"
tracing-subscriber = "0.3.18"
```

We set the version of Tokio to `=1.22.0`, this indicates that we want exactly this version. By
default, `cargo` would take any `1.x` version where `x` is greater than or equal to 22.

Now let's look at the output (truncated a little bit to remove things that we won't be focusing
on).

<pre data-lang="custom" style="background-color:#2b303b;color:#c0c5ce;" class="language-custom "><code class="language-custom" data-lang="custom"><span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-30</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:43:24</span></b></span><span style='opacity:0.67'>.010351Z</span> <span style='color:#9d4edd'>TRACE</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=, task.id=18, loc.file=&quot;src/main.rs&quot;, loc.line=10, loc.col=5}</span> <b><u><span style='color:#5aba84'>new</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-30</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:43:24</span></b></span><span style='opacity:0.67'>.010695Z</span> <span style='color:#9d4edd'>TRACE</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=, task.id=18, loc.file=&quot;src/main.rs&quot;, loc.line=10, loc.col=5}</span> <b><u><span style='color:#5aba84'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-30</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:43:24</span></b></span><span style='opacity:0.67'>.010778Z</span> <span style='color:#489e6c'> INFO</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=, task.id=18, loc.file=&quot;src/main.rs&quot;, loc.line=10, loc.col=5}</span> <b><span style='color:#aaa'>debugging_tokio_instrumentation</span></b>: <span style='color:#aaa'>fun=true pre-yield</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-30</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:43:24</span></b></span><span style='opacity:0.67'>.010829Z</span> <span style='color:#9d4edd'>TRACE</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=, task.id=18, loc.file=&quot;src/main.rs&quot;, loc.line=10, loc.col=5}</span> <b><span style='color:#c77dff'>tokio::task::waker</span></b>: <span style='color:#9d4edd'>op=&quot;waker.wake_by_ref&quot;, task.id=1</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-30</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:43:24</span></b></span><span style='opacity:0.67'>.010878Z</span> <span style='color:#9d4edd'>TRACE</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=, task.id=18, loc.file=&quot;src/main.rs&quot;, loc.line=10, loc.col=5}</span> <b><u><span style='color:#5aba84'>exit</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-30</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:43:24</span></b></span><span style='opacity:0.67'>.010924Z</span> <span style='color:#9d4edd'>TRACE</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=, task.id=18, loc.file=&quot;src/main.rs&quot;, loc.line=10, loc.col=5}</span> <b><u><span style='color:#5aba84'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-30</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:43:24</span></b></span><span style='opacity:0.67'>.010962Z</span> <span style='color:#9d4edd'>TRACE</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=, task.id=18, loc.file=&quot;src/main.rs&quot;, loc.line=10, loc.col=5}</span> <b><u><span style='color:#5aba84'>exit</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-30</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:43:24</span></b></span><span style='opacity:0.67'>.010997Z</span> <span style='color:#9d4edd'>TRACE</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=, task.id=18, loc.file=&quot;src/main.rs&quot;, loc.line=10, loc.col=5}</span> <b><u><span style='color:#5aba84'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-30</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:43:24</span></b></span><span style='opacity:0.67'>.011032Z</span> <span style='color:#9d4edd'>TRACE</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=, task.id=18, loc.file=&quot;src/main.rs&quot;, loc.line=10, loc.col=5}</span> <b><u><span style='color:#5aba84'>exit</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-30</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:43:24</span></b></span><span style='opacity:0.67'>.011065Z</span> <span style='color:#9d4edd'>TRACE</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=, task.id=18, loc.file=&quot;src/main.rs&quot;, loc.line=10, loc.col=5}</span> <b><u><span style='color:#5aba84'>close</span></u></b>
</code></pre>

Unfortunately it's way to wide to visualise nicely on this web-site. But let's walk through it.

The date and time and log level is pretty straight forward. I took the log level colours from the
[`fmt::Subscriber`], so those should be familiar.

### trace types 

All the lines in the output are prefixed with a span named `runtime.spawn`. Spans with this name
instrument tasks, `ari-subscriber` colours them green. There are district types of instrumentation
in Tokio, and they each get their own colour.

* <span style='color:#489e6c;background-color:#2b303b;padding:2px'>runtime.spawn</span> spans (green) instrument tasks
* <span style='color:#ba5a57;background-color:#2b303b;padding:2px'>runtime.resource</span> spans (red) instrument resources
* <span style='color:#5c8dce;background-color:#2b303b;padding:2px'>runtime.resource.async_op</span> spans (blue) instrument async operations
* <span style='color:#e5e44d;background-color:#2b303b;padding:2px'>runtime.resource.async_op.poll</span> spans (yellow) instrument the individual polls on async operations
* <span style='color:#c77dff;background-color:#2b303b;padding:2px'>tokio::task::waker</span> events (purple) represent discrete waker operations
* <span style='color:#ff9f1c;background-color:#2b303b;padding:2px'>runtime::resource::poll_op</span> events (orange) represent poll state changes
* <span style='color:#ff4d6d;background-color:#2b303b;padding:2px'>runtime::resource::state_update</span> events (pink) represent resource state changes
* <span style='color:#68d8d6;background-color:#2b303b;padding:2px'>runtime::resource::async_op::state_update</span> events (turquoise) represent async operation state changes

In the case of spans, the value given above is the span name, for events it is the target.

Describing how each of these traces is used within Tokio and how to interpret them would fill
several more posts and I won't go into that topic in more detail here. I already wrote a post
on the instrumentation for tasks, which covers the `runtime.spawn` spans and the
`tokio::task::waker` events. Go read [tracing tokio tasks](@/posts/tracing-tokio-tasks.md) to
learn about those!

### span events

Now let's get back to the output of `ari-subscriber` for our test program. The first line ends in
<b><u><span style='color:#5aba84'>new</span></u></b>, this is an event representing the creation of a
new span. There are equivalent lines for `enter`, `exit`, and `close`; all parts of the span
lifecycle. See the [span lifecycle](@/posts/tracing-tokio-tasks.md#span-lifecycle) section of the
post I linked above for a refresher on the lifecycle.

By default, the [`fmt::Subscriber`] doesn't output these "span events", but it can be configured to
do so with the [`with_span_events()`] method on the builder. Currently `ari-subscriber` always
emits these span events, but I may wish to make this configurable in the future to reduce the
amount of output.

[`with_span_events()`]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/struct.SubscriberBuilder.html#method.with_span_events

### analysing wakes

Let's find our wake operation. You'll notice that there is exactly one line at INFO level. This is
the one that we added to our spawned task ourselves. After the `runtime.spawn` span we see the text

```
debugging_tokio_instrumentation: fun=true pre-yield
```

The first bit (`debugging_tokio_instrumentation`) is the target, which by default is the same as
the module path so it's the name of our example application. After the colon are the fields (just
one field: `fun=true`) and finally the message (`pre-yield`). An event's message is actually just a
specially handled field with the name `message`. This event isn't coloured because it isn't part of
the instrumentation that `ari-subscriber` knows about. 

The next line is the wake operation (it's purple!). We can see that its target is
`tokio::task::waker` and then it has 2 fields and no message. The fields are
`op="waker.wake_by_ref"` and `task.id=1`. 

Let's start with the second field, `task.id=1`. This gives the **instrumentation ID** of the task
being woken. The instrumentation ID of a task is not the Tokio [`task::Id`], but rather the tracing
[`span::Id`] of the span which instruments that task. That value is the one that appears in
brackets after the span name `runtime.spawn` (e.g. `[1]`). This is a bit confusing, because the
`runtime.spawn` span also has a field called `task.id`, but that one refers to the Tokio task ID.
The important point here is that our span IDs match (both 1), so this operation is being performed
from within the task that it is affecting.

[`task::Id`]: https://docs.rs/tokio/1.35.1/tokio/task/struct.Id.html

The operation `wake_by_ref` indicates that the task is being woken using a reference to the waker.
This operation doesn't consume the waker - which is important when Tokio Console counts the number
of wakers for a given task to make sure that it hasn't lost all its wakers.

With this information, we can now manually ascertain that this is a self-wake operation. We are waking
a task while running within that task.

### what happens next

Let's change our version of Tokio to the latest (at the time of writing), 1.35.1.

```toml
tokio = { version = "=1.35.1", features = ["full", "tracing"] }
```

And now run exactly the same example. The output is below (truncated in the same way as before).

<pre data-lang="custom" style="background-color:#2b303b;color:#c0c5ce;" class="language-custom "><code class="language-custom" data-lang="custom"><span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-30</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>16:00:09</span></b></span><span style='opacity:0.67'>.484496Z</span> <span style='color:#9d4edd'>TRACE</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=, task.id=18, loc.file=&quot;src/main.rs&quot;, loc.line=10, loc.col=5}</span> <b><u><span style='color:#5aba84'>new</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-30</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>16:00:09</span></b></span><span style='opacity:0.67'>.484798Z</span> <span style='color:#9d4edd'>TRACE</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=, task.id=18, loc.file=&quot;src/main.rs&quot;, loc.line=10, loc.col=5}</span> <b><u><span style='color:#5aba84'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-30</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>16:00:09</span></b></span><span style='opacity:0.67'>.484867Z</span> <span style='color:#489e6c'> INFO</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=, task.id=18, loc.file=&quot;src/main.rs&quot;, loc.line=10, loc.col=5}</span> <b><span style='color:#aaa'>debugging_tokio_instrumentation</span></b>: <span style='color:#aaa'>fun=true pre-yield</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-30</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>16:00:09</span></b></span><span style='opacity:0.67'>.484930Z</span> <span style='color:#9d4edd'>TRACE</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=, task.id=18, loc.file=&quot;src/main.rs&quot;, loc.line=10, loc.col=5}</span> <b><span style='color:#c77dff'>tokio::task::waker</span></b>: <span style='color:#9d4edd'>op=&quot;waker.clone&quot;, task.id=1</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-30</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>16:00:09</span></b></span><span style='opacity:0.67'>.484998Z</span> <span style='color:#9d4edd'>TRACE</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=, task.id=18, loc.file=&quot;src/main.rs&quot;, loc.line=10, loc.col=5}</span> <b><u><span style='color:#5aba84'>exit</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-30</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>16:00:09</span></b></span><span style='opacity:0.67'>.485073Z</span> <span style='color:#9d4edd'>TRACE</span> <b><span style='color:#c77dff'>tokio::task::waker</span></b>: <span style='color:#9d4edd'>op=&quot;waker.wake&quot;, task.id=1</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-30</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>16:00:09</span></b></span><span style='opacity:0.67'>.485150Z</span> <span style='color:#9d4edd'>TRACE</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=, task.id=18, loc.file=&quot;src/main.rs&quot;, loc.line=10, loc.col=5}</span> <b><u><span style='color:#5aba84'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-30</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>16:00:09</span></b></span><span style='opacity:0.67'>.485208Z</span> <span style='color:#9d4edd'>TRACE</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=, task.id=18, loc.file=&quot;src/main.rs&quot;, loc.line=10, loc.col=5}</span> <b><u><span style='color:#5aba84'>exit</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-30</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>16:00:09</span></b></span><span style='opacity:0.67'>.485261Z</span> <span style='color:#9d4edd'>TRACE</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=, task.id=18, loc.file=&quot;src/main.rs&quot;, loc.line=10, loc.col=5}</span> <b><u><span style='color:#5aba84'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-30</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>16:00:09</span></b></span><span style='opacity:0.67'>.485313Z</span> <span style='color:#9d4edd'>TRACE</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=, task.id=18, loc.file=&quot;src/main.rs&quot;, loc.line=10, loc.col=5}</span> <b><u><span style='color:#5aba84'>exit</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-01-30</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>16:00:09</span></b></span><span style='opacity:0.67'>.485361Z</span> <span style='color:#9d4edd'>TRACE</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=, task.id=18, loc.file=&quot;src/main.rs&quot;, loc.line=10, loc.col=5}</span> <b><u><span style='color:#5aba84'>close</span></u></b>
</code></pre>

It might not be immediately obvious, but that output is one line longer than the previous one. What
jumps out is probably that we can now see a wake operation without scrolling to the right. But
first, let's check what happens above that.

Directly below our own `fun=true pre-yield` event line, we see that there is still a
`tokio::task::waker` event and it is still operating on the same task (and the same task that we
are currently inside), the one with the task instrumentation ID of 1. However, this isn't a wake
operation, instead it has the field value `op=waker.clone`. Somewhere, the waker for that task is
being cloned.

Straight afterwards we see that the span exits - which means that the call to poll on that task has
returned. After that, the task **is** woken. We see that the operation is `waker.wake` instead of
`waker.wake_by_ref`, which means that the waker is consumed (this makes sense, as it was cloned
before). More importantly than all of that though, is that this wake operation isn't inside the
`runtime.spawn` span for that task, in fact it isn't inside any spans at all, `runtime.spawn` or
otherwise.

This confirms what could be observed in Tokio Console, the instrumentation indicates that this is
not a self wake!

### what changed?

The reason for this change is the PR [`tokio#5223`] (in Tokio itself). This PR changes the
behaviour of [`yield_now()`] to defer the wake. When a task yields to the runtime in this way, it
is immediately ready to be polled again (that's the whole point). Any other task which is ready
will get precedence to be polled first (except under some specific conditions involving the LIFO
slot). However the scheduler won't necessarily poll the resource drivers, this means that a task
that is always ready may starve the resource drivers despite doing its best to be well behaved by
yielding regularly to the runtime.

[`tokio#5223`]: https://github.com/tokio-rs/tokio/pull/5223

The PR changes the behaviour to defer waking tasks which call [`yield_now()`] until after polling
the resource drivers, avoiding the starvation issue.

After some discussion on [console#512], we decided that it's OK that Tokio Console can't detect
this specific case of self wakes, since the PR on Tokio made them much less likely to result in
some performance issue - something which may still occur from other futures self waking.

And that's how I managed to use my very basic subscriber crate to answer a question quicker thanks
to pretty colours.

## should I use `ari-subscriber`?

Now that we've had a bit of a look at `ari-subscriber`, the big question is, should anyone be using
it?

The answer is **no**.

Aside from missing a bunch of useful features, `ari-subscriber` currently does a lot of things "the
easy way", which is not very performant. I know how to make it more performant, but I promised
myself I'd write this post before doing any more work on the crate.

Unless you too are trying to debug the instrumentation built into Tokio, you're much better off using
the [`fmt::Subscriber`] from the `tracing-subscriber` crate.

If you **are** debugging that instrumentation, please [come and say hi](@/about.md#contact)! I'd
be really interested to hear what you're doing and I might even be able to help.

### thanks

Thanks to [Luca Palmieri](https://lpalmieri.com/) and [One](https://github.com/c-git) for feedback
on the post!
