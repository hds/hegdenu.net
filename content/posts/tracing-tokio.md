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

### span lifetime

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

## tokio's instrumentation

As I mentioned at the beginning, Tokio is instrumented with Tracing.

It would be nice to see what's going on in there.

So let's write a very small async Rust program.

And look at the instrumentation.