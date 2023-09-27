+++
title = "tracing tokio"
slug = "tracing-tokio"
author = "hds"
date = "2023-09-05"
draft = true
+++

Async programming can be tricky.

Part of the reason for this is that your program is no longer linear.

Things start.

Then they pause.

Something else runs.

It pauses.

The first thing starts again.

The second thing runs.

Then it finishes.

Then the first thing pauses again.

What a mess!








[Tokio](https://tokio.rs/) is currently the most popular async runtime in Rust.












Wouldn't it be nice if you could visualise what your async program is doing?

The good news is you can!

(the not so good news is that the visualisation could be improved.)

How is this possible?

Tokio is instrumented!

Parts of the Tokio codebase are instrumented with [Tracing](https://github.com/tokio-rs/tracing).

## aside: tracing

Tracing is really an ecosystem all to itself.

At the core, Tracing is all about recording spans and events.

An event is something that happens at a specific moment in time.

(traditional logging is all events)

A span represents a period in time.

It doesn't have a single time.

It has two.

The start time and the end time.

(small lie, more on this later)

Let's look at a picture to illustrate the concept.

(pun totally intended)

![Time diagram showing an event as an instant in time (at time=1) and a span starting at time=3 and ending at time=6. There is another event inside the span at time=5.](/img/tracing-tokio/tracing-events_span.png)

Here we can see two events and a span.

(in a very simplistic visualisation)

The first event occurs at time=1.

(it is cleverly named `event 1`)

Then we have a span which starts time=3 and ends at time=6.

Within the span we have another event that occurs at time=5.

So we see that events can occur **within** the context of a span.

Spans can also occur within other spans.

Why do we care?

### fields

Traditional logging frameworks generally take two pieces of input for a given log.

The level.

(error, warn, info, debug, etc.)

The message

(some blob of text)

It is then up to whatever aggregates the logs to work out what is what.

(often the whatever is a person staring at their terminal)

This made sense when you logged to a byte stream of some sort.

(a file, stdout, etc.)

However we often produce too many logs for humans now.

So we want to optimise for machine readability.

This is where structured logging is useful.

Rather than spitting out any old text and then worrying about ingesting it.

We can write logs that can be easily parsed.

To illustrate, let's look at a prime example of non-structured logs.

I stole this example from a [Honeycomb blog post](https://www.honeycomb.io/blog/how-are-structured-logs-different-from-events).

```
Jun 20 09:51:47 cobbler com.apple.xpc.launchd[1] (com.apple.preference.displays.MirrorDisplays): Service only ran for 0 seconds. Pushing respawn out by 10 seconds.
```

Now let's reimagined this line as a structured log.

(again, courtesy of the good folk at Honeycomb)

```json
{"time":"Jun 20 09:51:47","hostname":"cobbler","process":"com.apple.xpc.launchd","pid":1,"service":"com.apple.preference.displays.MirrorDisplays","action":"respawn","respawn_delay_sec":10,"reason":"service stopped early","service_runtime_sec":0}
```

This isn't so readable for **you**.

But it's **so** much more readable for a machine.

Which brings us back to fields.

Tracing events and spans have fields.

So that the other side of the ecosystem can output nice structured logs.

(the other side being Tracing subscribers)

Or even send them to distributed tracing system.

So you can match up what this machine is doing with what some other machines are doing.

And that's the great thing about spans.

A spans children effectively inherit its fields.

So if you set a request id on a span.

(for example)

Then the children spans and events will have access to it.

How that is used is up to the subscriber.

### span lifecycle

It's now time to clear up that little lie.

The one about spans having a start and end time.

In Tracing, a span has a whole lifecycle.

It is created.

Then it is entered.

(this is when a span is active)

Then the span exits.

Now the span can enter and exit more times.

Finally the span closes.

![Time diagram showing the span lifecycle. The span is created (inactive), later entered and exited twice (so there are 2 active sections). Some time later it is closed.](/img/tracing-tokio/tracing-span_lifecycle.png)

The default `fmt` subscriber can give you the total busy and idle time for a span when it closes.

(that's from the [`tracing-subscriber`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/index.html) crate)

(use [.with_span_events()](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/struct.Layer.html#method.with_span_events) to enable this behaviour)

Later you'll see why knowing about span lifecycles is useful.

## tracing our code

As I mentioned at the beginning, Tokio is instrumented with Tracing.

It would be nice to see what's going on in there.

So let's write a very small async Rust program.

And look at the instrumentation.

The code is in the web-site repo: [tracing-tokio](https://github.com/hds/hegdenu.net/tree/main/resources/tracing-tokio).

We'll start with `Cargo.toml`

```toml
[package]
name = "tracing-tokio"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.32.0", features = ["full"] }
tracing = "0.1.37"
tracing-subscriber = "0.3.17"
```

Pretty straight forward list of dependencies.

(I'm including the exact version here, which isn't common)

(but hopefully helps anyone following along in the future)

We're looking at Tokio, so we'll need that.

We want to use Tracing too.

And to actually output our traces, we need the `tracing-subscriber` crate.

Now here's the code.

```rust
#[tokio::main]
async fn main() {
    // we will fill this in later!
    tracing_init();

    tokio::spawn(async {
        tracing::info!("step 1");

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        tracing::info!("step 2");
    })
    .await
    .expect("joining task failed");
}
```

OK, let's dig in.

We're using `#[tokio::main]` so we're in an async context from the beginning.

We set up tracing.

(we'll get into exactly how later)

Then we spawn a task.

Before we look at the contents of the task, look down.

We're awaiting the join handle returned by `spawn()`.

(so the task has to end before our program quits)

Now back into the task contents.

We record a tracing event with the message `"step 1"`.

(it's at info level)

Then we async sleep for 100ms.

Then record another tracing event.

This time with the message `"step 2"`.

### tracing init

Let's write a first version of our `init_tracing()` function.

```rust
fn tracing_init() {
    use tracing::Level;
    use tracing_subscriber::{filter::FilterFn, fmt::format::FmtSpan, prelude::*};

    let fmt_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_span_events(FmtSpan::FULL)
        .with_filter(FilterFn::new(|metadata| {
            metadata.target() == "tracing_tokio"
        }));
    tracing_subscriber::registry().with(fmt_layer).init();
}
```

Both the [`tracing`](https://docs.rs/tracing/latest/tracing/) and [`tracing-subscriber`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/) crates have extensive documentation.

So I won't go into too much depth.

We're setting up a formatting layer.

(think of a `tracing-subscriber` layer as a way to get traces out of your program)

(out and into the world!)

The `fmt` layer in `tracing-subscriber` will write your traces to the console.

Or to a file, or some other writer.

The `fmt` layer is really flexible in many ways.

We're going to use some of those.

We want [`.pretty()`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/struct.Layer.html#method.pretty) output.

This is a multi-line output which is easier to read on this web-site.

(I never use this normally)

The call to `.with_span_events()` won't do anything just yet.

(so we'll skip it now and discuss later)

Finally we have a [`.with_filter()`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/trait.Layer.html#method.with_filter).

For now we only want the spans from our own crate.

(that was simple, right?)

Let's look at the result.

<pre class="bespoke-code"><code>  <span style='opacity:0.67'>2023-09-27T13:46:56.852712Z</span> <span style='color:#0a0'> INFO</span> <b><span style='color:#0a0'>tracing_tokio</span></b><span style='color:#0a0'>: step 1</span>
    <span style='opacity:0.67'><i>at</i></span> resources/tracing-tokio/src/main.rs:29

  <span style='opacity:0.67'>2023-09-27T13:46:56.962809Z</span> <span style='color:#0a0'> INFO</span> <b><span style='color:#0a0'>tracing_tokio</span></b><span style='color:#0a0'>: step 2</span>
    <span style='opacity:0.67'><i>at</i></span> resources/tracing-tokio/src/main.rs:33</code></pre>

We logs for each of our two events.

And they're roughly 100ms apart.

(it's actually more like 110ms)

(I wonder where that time went?)

(today we don't care)


OK, let's start tracing something inside Tokio.

There are a few things we have to do here.

1. include task spawn spans in our filter

2. enable the `tracing` feature in tokio

3. build with the `tokio_unstable` cfg flag

The filter is straight forward.

To include the spans, we update the filter.

Our `fmt` layer creation will now look like the following.

```rust
    let fmt_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_span_events(FmtSpan::FULL)
        .with_filter(FilterFn::new(|metadata| {
            if metadata.target() == "tracing_tokio" {
                true
            } else if metadata.target() == "tokio::task" && metadata.name() == "runtime.spawn" {
                true
            } else {
                false
            }
        }));
```

Which is to say.

We also accept the `tokio::task` target.

But only if the span's name is `runtime.spawn`.

Now let's add the `tracing` feature.

This is as simple as modifying the tokio line in `Cargo.toml`.

```toml
tokio = { version = "1.32", features = ["full", "tracing"] }
```

Finally, `tokio_unstable`.

### aside: `tokio_unstable`

Tokio takes [semantic versioning](https://semver.org/) seriously.

(like most Rust projects)

Tokio is now past version 1.0.

This means that no breaking changes should be included without going to version 2.0.

That would seriously fragment Rust's async ecosystem.

So it's unlikely to happen.

An escape hatch is `tokio_unstable`.

Anything behind `tokio_unstable` is considered fair game to break between minor releases.

This doesn't mean that the code is necessarily less tested.

(although some of it hasn't been as extensively profiled)

But it isn't guaranteed to be stable.

I know of some very intensive workloads that are run with `tokio_unstable` builds.

So, how do we enable it?

We need to pass `--cfg tokio_unstable` to `rustc`.

The easiest way to do this is to add the following to `.cargo/config` in your crate root.

```toml
[build]
rustflags = ["--cfg", "tokio_unstable"]
```

(needs to be in the workspace root if you're in a workspace)

(otherwise it won't do nuthin')

Back to tracing!

## tracing tasks

Each time a task is spawned, a span is created.

When the task is polled, the span is entered.

When the poll ends, the span is exited again.

This way a task spawn span may be entered multiple times.

When the task is dropped, the span is closed.

(if you want to understand what polling is, I have a blog series for that)

(check out [how I finally understood async/await in Rust](@/posts/understanding-async-await-1.md))

We'd like to see all these steps in the [span lifecycle](#span-lifecycle) in our logs.

Which is where span events come in.

By default the `fmt` layer doesn't output lines for spans.

Just events and the spans they're inside.

Span events are the way to get lines about spans themselves.

Specifically, events for each stage of a span's lifecycle.

(new span, enter, exit, and close)

We enable span events using [`.with_span_events()`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/struct.Layer.html#method.with_span_events).

Now we're ready!

Let's see the output we get now.

<pre class="bespoke-code"><code>  <span style='opacity:0.67'>2023-09-27T13:32:49.609363Z</span> <span style='color:#a0a'>TRACE</span> <b><span style='color:#a0a'>tokio::task</span></b><span style='color:#a0a'>: new</span>
    <span style='opacity:0.67'><i>at</i></span> /Users/stainsby/.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.32.0/src/util/trace.rs:17
    <span style='opacity:0.67'><i>in</i></span> tokio::task::<b>runtime.spawn</b> <span style='opacity:0.67'><i>with</i></span> <b>kind</b>: task, <b>task.name</b>: , <b>task.id</b>: 18, <b>loc.file</b>: &quot;resources/tracing-tokio/src/main.rs&quot;, <b>loc.line</b>: 25, <b>loc.col</b>: 9

  <span style='opacity:0.67'>2023-09-27T13:32:49.615907Z</span> <span style='color:#a0a'>TRACE</span> <b><span style='color:#a0a'>tokio::task</span></b><span style='color:#a0a'>: enter</span>
    <span style='opacity:0.67'><i>at</i></span> /Users/stainsby/.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.32.0/src/util/trace.rs:17
    <span style='opacity:0.67'><i>in</i></span> tokio::task::<b>runtime.spawn</b> <span style='opacity:0.67'><i>with</i></span> <b>kind</b>: task, <b>task.name</b>: , <b>task.id</b>: 18, <b>loc.file</b>: &quot;resources/tracing-tokio/src/main.rs&quot;, <b>loc.line</b>: 25, <b>loc.col</b>: 9

  <span style='opacity:0.67'>2023-09-27T13:32:49.621633Z</span> <span style='color:#0a0'> INFO</span> <b><span style='color:#0a0'>tracing_tokio</span></b><span style='color:#0a0'>: step 1</span>
    <span style='opacity:0.67'><i>at</i></span> resources/tracing-tokio/src/main.rs:26
    <span style='opacity:0.67'><i>in</i></span> tokio::task::<b>runtime.spawn</b> <span style='opacity:0.67'><i>with</i></span> <b>kind</b>: task, <b>task.name</b>: , <b>task.id</b>: 18, <b>loc.file</b>: &quot;resources/tracing-tokio/src/main.rs&quot;, <b>loc.line</b>: 25, <b>loc.col</b>: 9

  <span style='opacity:0.67'>2023-09-27T13:32:49.627000Z</span> <span style='color:#a0a'>TRACE</span> <b><span style='color:#a0a'>tokio::task</span></b><span style='color:#a0a'>: exit</span>
    <span style='opacity:0.67'><i>at</i></span> /Users/stainsby/.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.32.0/src/util/trace.rs:17
    <span style='opacity:0.67'><i>in</i></span> tokio::task::<b>runtime.spawn</b> <span style='opacity:0.67'><i>with</i></span> <b>kind</b>: task, <b>task.name</b>: , <b>task.id</b>: 18, <b>loc.file</b>: &quot;resources/tracing-tokio/src/main.rs&quot;, <b>loc.line</b>: 25, <b>loc.col</b>: 9

  <span style='opacity:0.67'>2023-09-27T13:32:49.728407Z</span> <span style='color:#a0a'>TRACE</span> <b><span style='color:#a0a'>tokio::task</span></b><span style='color:#a0a'>: enter</span>
    <span style='opacity:0.67'><i>at</i></span> /Users/stainsby/.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.32.0/src/util/trace.rs:17
    <span style='opacity:0.67'><i>in</i></span> tokio::task::<b>runtime.spawn</b> <span style='opacity:0.67'><i>with</i></span> <b>kind</b>: task, <b>task.name</b>: , <b>task.id</b>: 18, <b>loc.file</b>: &quot;resources/tracing-tokio/src/main.rs&quot;, <b>loc.line</b>: 25, <b>loc.col</b>: 9

  <span style='opacity:0.67'>2023-09-27T13:32:49.735361Z</span> <span style='color:#0a0'> INFO</span> <b><span style='color:#0a0'>tracing_tokio</span></b><span style='color:#0a0'>: step 2</span>
    <span style='opacity:0.67'><i>at</i></span> resources/tracing-tokio/src/main.rs:30
    <span style='opacity:0.67'><i>in</i></span> tokio::task::<b>runtime.spawn</b> <span style='opacity:0.67'><i>with</i></span> <b>kind</b>: task, <b>task.name</b>: , <b>task.id</b>: 18, <b>loc.file</b>: &quot;resources/tracing-tokio/src/main.rs&quot;, <b>loc.line</b>: 25, <b>loc.col</b>: 9

  <span style='opacity:0.67'>2023-09-27T13:32:49.741740Z</span> <span style='color:#a0a'>TRACE</span> <b><span style='color:#a0a'>tokio::task</span></b><span style='color:#a0a'>: exit</span>
    <span style='opacity:0.67'><i>at</i></span> /Users/stainsby/.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.32.0/src/util/trace.rs:17
    <span style='opacity:0.67'><i>in</i></span> tokio::task::<b>runtime.spawn</b> <span style='opacity:0.67'><i>with</i></span> <b>kind</b>: task, <b>task.name</b>: , <b>task.id</b>: 18, <b>loc.file</b>: &quot;resources/tracing-tokio/src/main.rs&quot;, <b>loc.line</b>: 25, <b>loc.col</b>: 9

  <span style='opacity:0.67'>2023-09-27T13:32:49.747868Z</span> <span style='color:#a0a'>TRACE</span> <b><span style='color:#a0a'>tokio::task</span></b><span style='color:#a0a'>: close, <b>time.busy</b></span><span style='color:#a0a'>: 24.4ms, <b>time.idle</b></span><span style='color:#a0a'>: 114ms</span>
    <span style='opacity:0.67'><i>at</i></span> /Users/stainsby/.cargo/registry/src/index.crates.io-6f17d22bba15001f/tokio-1.32.0/src/util/trace.rs:17
    <span style='opacity:0.67'><i>in</i></span> tokio::task::<b>runtime.spawn</b> <span style='opacity:0.67'><i>with</i></span> <b>kind</b>: task, <b>task.name</b>: , <b>task.id</b>: 18, <b>loc.file</b>: &quot;resources/tracing-tokio/src/main.rs&quot;, <b>loc.line</b>: 25, <b>loc.col</b>: 9</code></pre>