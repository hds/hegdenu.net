+++
title = "tracing tokio resources"
slug = "tracing-tokio-resources"
author = "hds"
date = "2024-08-07"
+++

Usually, I like to write posts about things I understand. Even if this means that I have to go and understand something first.

This post is going to be a bit different. Instead, we're going to follow along as I try to understand something.

One of my favourite topics to write about is the tracing instrumentation in Tokio and how it can be used to better understand your asynchronous code. I've previously written a bit about the tracing instrumentation for tasks: ([tracing tokio tasks](@/posts/tracing-tokio-tasks.md), [tokio waker instrumentation](@/posts/tokio-waker-instrumentation.md), also mentioned in [debugging tokio instrumentation](@/posts/debugging-tokio-instrumentation.md) and [scheduled time](@/posts/scheduled-time.md)). But there is a whole other side to the tracing in tokio which instruments resources.

A resource in this context is an async primitive provided by Tokio which can be used as a building block. Timers are resources, as are most of the types in the [`tokio::sync`](https://docs.rs/tokio/1/tokio/sync/index.html) module, such as mutexes, semaphores, and channels. (Important caveat: as of today, the only channel that is instrumented is the [`oneshot`](https://docs.rs/tokio/1/tokio/sync/oneshot/index.html) channel).

I've spent a bit of time working with the tracing instrumentation in Tokio, but I will admit that I've never really understood the structure of the resource instrumentation - it's significantly more complicated than the structure of the task instrumentation. So let's jump into whatever sources we can find to help us understand how resources are instrumented.

The code is a good starting point. The instrumentation in Tokio was built specifically for (and at the same time as) Tokio Console, so it makes sense to look in both places. All the links to actual code will be based on the following versions:
- [Tokio v1.39.1](https://github.com/tokio-rs/tokio/tree/tokio-1.39.1)
- [Console Subscriber v0.4.0](https://github.com/tokio-rs/console/tree/console-subscriber-v0.4.0)

However, the code can be a bit overwhelming. So let's look at the PRs when this instrumentation was added.

The first pair of PRs are when [`tokio::time::Sleep`](https://docs.rs/tokio/1/tokio/time/struct.Sleep.html) was instrumented and the code to read resource instrumentation was added to the `console-subscriber`:
- **tracing: instrument time::Sleep** ([tokio-rs/tokio#4072](https://github.com/tokio-rs/tokio/pull/4072))
- **feat(subscriber): resource instrumentation** ([tokio-rs/console#77](https://github.com/tokio-rs/console/pull/77))

The next pair are when further resources (all from `tokio::sync`) were instrumented and when the resource detail view was added to Tokio Console:
- **tracing: instrument more resources** ([tokio-rs/tokio#4302](https://github.com/tokio-rs/tokio/pull/4302))
- **feature: add resource detail view** ([tokio-rs/console#188](https://github.com/tokio-rs/console/pull/188))

After these PRs, no further resources were instrumented. Interestingly (but perhaps not surprisingly in the world of Open Source Software), they were all written by the one person: [Zahari Dichev](https://github.com/zaharidichev). Unfortunately, Zahari isn't active in the Tokio project these days, but if I could get a hold of him, I would love to pick his brain about this work.

I checked with [Eliza](https://github.com/hawkw) about whether there was any more written up about the strategy behind resource instrumentation, but unfortunately there isn't. This wasn't necessarily a given, because there **is** a fantastic write-up by [Felix S Klock II](https://github.com/pnkfelix) which describes much of the vision that Tokio Console (and hence the instrumentation in Tokio itself) hope to achieve. Road to TurboWish ([part 1](http://blog.pnkfx.org/blog/2021/04/26/road-to-turbowish-part-1-goals/), [part 2](http://blog.pnkfx.org/blog/2021/04/27/road-to-turbowish-part-2-stories/), [part 3](http://blog.pnkfx.org/blog/2021/05/03/road-to-turbowish-part-3-design/)).

Now that we know that there isn't any easy source for this information, let's go through the code and make use of my [`ari-subscriber`](https://docs.rs/ari-subscriber/latest/ari_subscriber/) crate to view the tracing output directly.

### a note on tone

As I mentioned above, this post was written while I was investigating and digging into how resource instrumentation works. There are some parts where I question the way things are implemented. However, I mean no disrespect to anyone involved in the implementation.

The instrumentation for resources and the visualisation for that instrumentation went into Tokio and Tokio Console over a small number of PRs, most of it is in the 4 PRs I linked earlier. This feature is incredible complete given that fact, and it's been running and doing its thing (in `tokio_unstable`) since then. With the benefit I have of looking back on this a number of years later, there might be things that I would change now, but it's not fair to expect anyone to have done differently at the time.

A huge thanks to [Zahari](https://github.com/zaharidichev) who wrote most of this, as well as [Eliza](https://github.com/hawkw), [Alice](https://github.com/Darksonn), [Sean](https://github.com/seanmonstar), [Carl](https://github.com/carllerche), and [Felix](https://github.com/pnkfelix) who reviewed. We have an incredible foundation from that work alone!

## oneshot resource

Let's begin with an example. It was harder to pick an example than I anticipated. It turns out that many resources operate via a "sub-resource", which makes things more confusing on first read through. For this reason, I went with the [`std::sync::oneshot`](https://docs.rs/tokio/1/tokio/sync/oneshot/index.html) channel - it's a fairly simple example, even if the code is a little longer than we may wish.

```rust
use tracing::debug;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    use tracing_subscriber::prelude::*;
    let layer = ari_subscriber::Layer::new();
    tracing_subscriber::registry().with(layer).init();

    spawn_named("tokio-oneshot", tokio_oneshot()).await.unwrap();
}

async fn tokio_oneshot() {
    debug!("creating oneshot");
    let (tx, rx) = tokio::sync::oneshot::channel::<u64>();
    debug!("oneshot created");

    let jh = spawn_named("receiver", async move {
        debug!("awaiting message");
        let msg = rx.await.unwrap();
        debug!(msg, "message received");
    });

    spawn_named("sender", async move {
        debug!("sending message");
        tx.send(5).unwrap();
        debug!("message sent");
    });

    debug!(?jh, "awaiting receiver task");
    jh.await.unwrap();
    debug!("good-bye");
}
```

Our application is async and starts up in a [`current_thread`](https://docs.rs/tokio/1/tokio/runtime/index.html#current-thread-scheduler) Tokio runtime. It's easier to see what is happening when we restrict everything interesting we do in the application to a single thread.

We start by setting up the `ari-subscriber` layer. The output shown in this post is from a slightly modified version. I hope to make some of these changes optional features of the published crate in the future, for now you can use them from the [ari-subscriber `tracing-tokio-tasks` branch](https://github.com/hds/ari-subscriber/tree/hegdenu/tracing-tokio-resources). It even includes the above code in the `examples` directory in [`tokio-oneshot.rs`](https://github.com/hds/ari-subscriber/blob/hegdenu/tracing-tokio-resources/examples/tokio-oneshot.rs).

We then spawn a task to run what would otherwise be the rest of our `main` function, but has been moved to its own function `tokio_oneshot()`. I've learnt to do this from previous posts, because this way we can easily see when this "main" task yields to the runtime.

In `tokio_oneshot()` we have 4 sections.
1. Create oneshot channel
2. Spawn `receiver` task which awaits the message from the receiver half of the channel
3. Spawn `sender` task that sends a message via the sender half of the channel
4. Await the `receiver` task and exit

Of course, this is an async application, so things won't necessarily run in that order. We wrap most of the interesting things we're doing in `debug!` events to make the important steps easier to separate. Notice that we only await the `receiver` task, this is because the `sender` task doesn't yield to the runtime until it completes, so it will always finish before the `receiver` task (which is awaiting the message it sends).

Before we look at the traces generated by this code, let's try and work out what kinds of spans and events we're going to find.

## instrumentation types

In previous posts, we focused on only the spans and events related to tasks and wakers, for which we had one type of span (for tasks) and one type of event (for wakers) respectively. For resources, there are a lot more.

Reading through the first of the 2 `console` PRs I linked above ([tokio-rs/console#77](https://github.com/tokio-rs/console/pull/77)), we can find all the span names and event targets for the different instrumentation that we're interested in. Those are in `lib.rs` in the implementation of [`Layer::register_callsite()`](https://github.com/tokio-rs/console/pull/77/files#diff-4dc28d2e391f111109f730ee4c47176297eebd5a3d60a4fb193e770fe1422b01R338-R348).

Fortunately, we already have an enumeration of these instrumentation types and what they correspond to built right into `ari-subscriber`. The blog post introducing it lists the [trace types](@/posts/debugging-tokio-instrumentation.md#trace-types). To save you following that link and because we're going to be looking at all of these in a moment, I'll repeat that list here:

- <span style='color:#489e6c;background-color:#2b303b;padding:2px'><code>runtime.spawn</code></span> spans (green) instrument tasks
- <span style='color:#ba5a57;background-color:#2b303b;padding:2px'><code>runtime.resource</code></span> spans (red) instrument resources
- <span style='color:#5c8dce;background-color:#2b303b;padding:2px'><code>runtime.resource.async_op</code></span> spans (blue) instrument async operations
- <span style='color:#e5e44d;background-color:#2b303b;padding:2px'><code>runtime.resource.async_op.poll</code></span> spans (yellow) instrument the individual polls on async operations
- <span style='color:#c77dff;background-color:#2b303b;padding:2px'><code>tokio::task::waker</code></span> events (purple) represent discrete waker operations
- <span style='color:#ff9f1c;background-color:#2b303b;padding:2px'><code>runtime::resource::poll_op</code></span> events (orange) represent poll state changes
- <span style='color:#ff4d6d;background-color:#2b303b;padding:2px'><code>runtime::resource::state_update</code></span> events (pink) represent resource state changes
- <span style='color:#68d8d6;background-color:#2b303b;padding:2px'><code>runtime::resource::async_op::state_update</code></span> events (turquoise) represent async operation state changes

We'll now spend the rest of the post getting a bit more familiar with most of these instrumentation types.

## channel creation

Let's start by looking at what instrumentation is produced when we create the oneshot channel. That corresponds to the code snippet below.

```rust
debug!("creating oneshot");
let (tx, rx) = tokio::sync::oneshot::channel::<u64>();
debug!("oneshot created");
```

This code produces the following traces, output by `ari-subscriber`.

{% traces() %}<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.586108Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=tokio-oneshot, task.id=2}</span>  <b><u><span style='color:#5aba84'>new</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.586396Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=tokio-oneshot, task.id=2}</span>  <b><u><span style='color:#5aba84'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.586456Z</span> <span style='color:#5c8dce'>DEBUG</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=tokio-oneshot, task.id=2}</span> 
     <span style='color:#aaa'>⤷</span> <b><span style='color:#aaa'>tokio_oneshot</span></b>: <span style='color:#aaa'>creating oneshot</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.586519Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span>  <b><u><span style='color:#df5853'>new</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.586592Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span>  <b><u><span style='color:#df5853'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.586645Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#c9184a'>⤷</span> <b><span style='color:#ff4d6d'>runtime::resource::state_update</span></b>: <span style='color:#c9184a'>tx_dropped=false, tx_dropped.op=&quot;override&quot;</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.586707Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span>  <b><u><span style='color:#df5853'>exit</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.586758Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span>  <b><u><span style='color:#df5853'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.586808Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#c9184a'>⤷</span> <b><span style='color:#ff4d6d'>runtime::resource::state_update</span></b>: <span style='color:#c9184a'>rx_dropped=false, rx_dropped.op=&quot;override&quot;</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.586867Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span>  <b><u><span style='color:#df5853'>exit</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.586914Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span>  <b><u><span style='color:#df5853'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.586964Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#c9184a'>⤷</span> <b><span style='color:#ff4d6d'>runtime::resource::state_update</span></b>: <span style='color:#c9184a'>value_sent=false, value_sent.op=&quot;override&quot;</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.587023Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span>  <b><u><span style='color:#df5853'>exit</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.587073Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span>  <b><u><span style='color:#df5853'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.587121Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#c9184a'>⤷</span> <b><span style='color:#ff4d6d'>runtime::resource::state_update</span></b>: <span style='color:#c9184a'>value_received=false, value_received.op=&quot;override&quot;</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.587179Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span>  <b><u><span style='color:#df5853'>exit</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.587229Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span>  <b><u><span style='color:#df5853'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.587279Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#5c8dce'>⤷</span> <span style='color:#5c8dce'>runtime.resource.async_op[<b><span style='color:#508ee3'>3</span></b></span><span style='color:#5c8dce'>]{source=&quot;Receiver::await&quot;}</span>  <b><u><span style='color:#508ee3'>new</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.587360Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span>  <b><u><span style='color:#df5853'>exit</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.587408Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#5c8dce'>⤷</span> <span style='color:#5c8dce'>runtime.resource.async_op[<b><span style='color:#508ee3'>3</span></b></span><span style='color:#5c8dce'>]{source=&quot;Receiver::await&quot;}</span>  <b><u><span style='color:#508ee3'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.587463Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#5c8dce'>⤷</span> <span style='color:#5c8dce'>runtime.resource.async_op[<b><span style='color:#508ee3'>3</span></b></span><span style='color:#5c8dce'>]{source=&quot;Receiver::await&quot;}</span> 
       <span style='color:#e5e44d'>⤷</span> <span style='color:#e5e44d'>runtime.resource.async_op.poll[<b><span style='color:#f5f466'>4</span></b></span><span style='color:#e5e44d'>]{}</span>  <b><u><span style='color:#f5f466'>new</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.587539Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#5c8dce'>⤷</span> <span style='color:#5c8dce'>runtime.resource.async_op[<b><span style='color:#508ee3'>3</span></b></span><span style='color:#5c8dce'>]{source=&quot;Receiver::await&quot;}</span>  <b><u><span style='color:#508ee3'>exit</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.587592Z</span> <span style='color:#5c8dce'>DEBUG</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=tokio-oneshot, task.id=2}</span> 
     <span style='color:#aaa'>⤷</span> <b><span style='color:#aaa'>tokio_oneshot</span></b>: <span style='color:#aaa'>oneshot created</span>
{% end %}

Oh wow! That's already a lot of traces. This may look a bit overwhelming, but we'll go through it bit by bit. The rest of it won't be so bad (we hope!).

Right up the beginning, we create a new task. This is something we've seen in previous posts. This is the task that was spawned from `main()` using the async function `tokio_oneshot()`. Let's look at those traces, up to our first debug message.

{% traces() %}<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.586108Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=tokio-oneshot, task.id=2}</span>  <b><u><span style='color:#5aba84'>new</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.586396Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=tokio-oneshot, task.id=2}</span>  <b><u><span style='color:#5aba84'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.586456Z</span> <span style='color:#5c8dce'>DEBUG</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=tokio-oneshot, task.id=2}</span> 
     <span style='color:#aaa'>⤷</span> <b><span style='color:#aaa'>tokio_oneshot</span></b>: <span style='color:#aaa'>creating oneshot</span>
{% end %}

That's not so bad. We create a new task, it gets polled for the first time, and then we emit the debug message (within the scope of the task, so it has the `runtime.spawn` span as its parent).

Now, let's look at the next little bit.

{% traces() %}<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.586519Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span>  <b><u><span style='color:#df5853'>new</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.586592Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span>  <b><u><span style='color:#df5853'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.586645Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#c9184a'>⤷</span> <b><span style='color:#ff4d6d'>runtime::resource::state_update</span></b>: <span style='color:#c9184a'>tx_dropped=false, tx_dropped.op=&quot;override&quot;</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.586707Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span>  <b><u><span style='color:#df5853'>exit</span></u></b>
{% end %}

Here we create a `runtime.resource` span. From our list above, we know that this span represents a resource. The kind is `Sync` and the concrete type is specified as `Sender|Receiver`. This doesn't really seem concrete enough to me, what if there are other resources that are concretely "sender & receiver"? But never mind, we know that this must represent our oneshot channel, so that's good enough for us.

We then enter the new `runtime.resource` span and emit a `runtime::resource::state_update` event. From our list, we know that this event represents a state change in the resource. This event has 2 fields:
- `tx_dropped=false`
- `tx_dropped.op=override`

This seems to mean that we want to override the value of the state `tx_dropped` in our resource by setting it to `false`. We can intuit that this means that the sender. `tx` is a common abbreviation for [signal transmission](https://en.wikipedia.org/wiki/Signal_transmission), which is to say, "sending", in fact we also used `tx` as the variable name for the sender half of our oneshot channel.

After emitting this event, we exit the `runtime.resource` span again.

Now that this all makes sense, we can look back at the larger snippet of traces that we started with. We see the same "enter span - emit event - exit span" pattern repeat another three times. Each time, we're overriding the value of a different bit of state. The other 3 are:
- `rx_dropped=false`
- `value_sent=false`
- `value_received=false`

This also makes sense. These values are a starting state for the channel. What is a little strange is that we enter and exit the spans each time. The code that produces these traces was introduced in the second of the 2 Tokio PRs I mentioned at the beginning of the post ([tokio-rs/tokio#4302](https://github.com/tokio-rs/tokio/pull/4302)). Let's look at the code today, it hasn't changed and can be found in [`oneshot.rs:485-515`](https://github.com/tokio-rs/tokio/blob/tokio-1.39.1/tokio/src/sync/oneshot.rs#L485-L515).

```rust
resource_span.in_scope(|| {
    tracing::trace!(
    target: "runtime::resource::state_update",
    tx_dropped = false,
    tx_dropped.op = "override",
    )
});

resource_span.in_scope(|| {
    tracing::trace!(
    target: "runtime::resource::state_update",
    rx_dropped = false,
    rx_dropped.op = "override",
    )
});

resource_span.in_scope(|| {
    tracing::trace!(
    target: "runtime::resource::state_update",
    value_sent = false,
    value_sent.op = "override",
    )
});

resource_span.in_scope(|| {
    tracing::trace!(
    target: "runtime::resource::state_update",
    value_received = false,
    value_received.op = "override",
    )
});
```

Now we see what's happening, the `runtime.resource` span is entered separately for each event. The reason that it's entered, is that the Console Subscriber uses the current context (which spans are active) to determine which `runtime.resource` span a `runtime::resource::state_update` event should be updating. I'm not sure why the span was entered separately for each one, I believe that this could be rewritten to enter the `runtime.resource` just once and then emit all events - but I'd have to test to be sure.

If the Console Subscriber were modified to use the traditional tracing span parent lookup, which is to first check for an explicit parent and only if one isn't set to check the context, then we could use the `parent:` directive in the events and we wouldn't need to enter the `runtime.resource` span at all. This was discussed on the PR where resource instrumentation was first introduced into Tokio ([tokio-rs/tokio#4072](https://github.com/tokio-rs/tokio/pull/4072)), but wasn't changed. Again, I'm not entirely sure why.

If we did that, then each of the `runtime::resource::state_update` events would look like this:

```rust
tracing::trace!(
    target: "runtime::resource::state_update",
    parent: resource_span,
    tx_dropped = false,
    tx_dropped.op = "override",
);
```

Maybe this is something we can look at in the future, but it would break older versions of the Console Subscriber (including the one that is the latest at the time of writing). It would avoid all the entering and exiting spans though.

After all the state updates (which I won't repeat all the traces for), the `runtime.resource.async_op` and `runtime.resource.async_op.poll` spans are created.

{% traces() %}<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.587229Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span>  <b><u><span style='color:#df5853'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.587279Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#5c8dce'>⤷</span> <span style='color:#5c8dce'>runtime.resource.async_op[<b><span style='color:#508ee3'>3</span></b></span><span style='color:#5c8dce'>]{source=&quot;Receiver::await&quot;}</span>  <b><u><span style='color:#508ee3'>new</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.587360Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span>  <b><u><span style='color:#df5853'>exit</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.587408Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#5c8dce'>⤷</span> <span style='color:#5c8dce'>runtime.resource.async_op[<b><span style='color:#508ee3'>3</span></b></span><span style='color:#5c8dce'>]{source=&quot;Receiver::await&quot;}</span>  <b><u><span style='color:#508ee3'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.587463Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#5c8dce'>⤷</span> <span style='color:#5c8dce'>runtime.resource.async_op[<b><span style='color:#508ee3'>3</span></b></span><span style='color:#5c8dce'>]{source=&quot;Receiver::await&quot;}</span> 
       <span style='color:#e5e44d'>⤷</span> <span style='color:#e5e44d'>runtime.resource.async_op.poll[<b><span style='color:#f5f466'>4</span></b></span><span style='color:#e5e44d'>]{}</span>  <b><u><span style='color:#f5f466'>new</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.587539Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#5c8dce'>⤷</span> <span style='color:#5c8dce'>runtime.resource.async_op[<b><span style='color:#508ee3'>3</span></b></span><span style='color:#5c8dce'>]{source=&quot;Receiver::await&quot;}</span>  <b><u><span style='color:#508ee3'>exit</span></u></b>
{% end %}

Here the `runtime.resource` span is entered again to create a `runtime.resource.async_op` span. In turn, the `runtime.resource.async_op` span is entered to create a `runtime.resource.async_op.poll` span. Once again, the reason that the parent span is entered before creating the next span is because the console subscriber depends on the current active span state to determine the parent of each of these spans.

Finally, we see our own debug message stating that the oneshot channel has been created.

{% traces() %}<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.587592Z</span> <span style='color:#5c8dce'>DEBUG</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=tokio-oneshot, task.id=2}</span> 
     <span style='color:#aaa'>⤷</span> <b><span style='color:#aaa'>tokio_oneshot</span></b>: <span style='color:#aaa'>oneshot created</span>
{% end %}

## spawning tasks

The next thing that we do is spawn the `receiver` and the `sender` tasks. If we take out the contents of the async blocks we pass to those tasks, that code looks like the following.

```rust
let jh = spawn_named("receiver", async move { .. });

spawn_named("sender", async move { .. });

debug!(?jh, "awaiting receiver task");
jh.await.unwrap();
```

Remember that while spawned tasks don't have to be awaited to start running, we're using a `current_thread` Tokio runtime, so until our `tokio_oneshot` task yields to the runtime, those new tasks won't get a chance to run.

The next lot of traces correspond to those lines of code.

{% traces() %}<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.587650Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>5</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=receiver, task.id=3}</span>  <b><u><span style='color:#5aba84'>new</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.587730Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>6</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=sender, task.id=4}</span>  <b><u><span style='color:#5aba84'>new</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.587807Z</span> <span style='color:#5c8dce'>DEBUG</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=tokio-oneshot, task.id=2}</span> 
     <span style='color:#aaa'>⤷</span> <b><span style='color:#aaa'>tokio_oneshot</span></b>: <span style='color:#aaa'>jh=JoinHandle { id: Id(3) } awaiting receiver task</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.587876Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=tokio-oneshot, task.id=2}</span> 
     <span style='color:#9d4edd'>⤷</span> <b><span style='color:#c77dff'>tokio::task::waker</span></b>: <span style='color:#9d4edd'>op=&quot;waker.clone&quot;, task.id=1</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.587940Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=tokio-oneshot, task.id=2}</span>  <b><u><span style='color:#5aba84'>exit</span></u></b>
{% end %}

We see that 2 new `runtime.spawn` spans are created, each one representing one of the tasks we're spawning. The first is the `receiver` task, the second is the `sender` task. As mentioned previously, we only store the join handle `jh` for the `receiver` task.

We've got a debug line in there before we await the join handle, and we're outputting the `Debug` value of that join handle as well. We can see that it's the join handle for Tokio task Id 3. Looking at the `task.id` field in the `resource.spawn` spans, we can see that 3 corresponds with the `receiver` task, as expected.

We then see the waker for `task.id=1` (for a waker event, this is the `resource.spawn` span ID). That span ID corresponds to the task we're currently in, the `tokio_oneshot` task. Then that same `runtime.spawn` span exits - the task it represents yields to the runtime. For more details on how to read waker events, you can read the post [tokio waker instrumentation](@/posts/tokio-waker-instrumentation.md).

### join handles are resources too

Not all resources in Tokio are instrumented. One of the ones that isn't is the [`JoinHandle`](https://docs.rs/tokio/1/tokio/task/struct.JoinHandle.html). While you may not think of a join handle as a resource, it really is. It allows one task to synchronise on when another task ends, as well as transporting the return value of the future that drives that task.

In the future, instrumenting the join handle would allow us to follow the dependence of one task on the completion of another!

## oneshot receiver task

Let's look at the code in the receiver task.

```rust
debug!("awaiting message");
let msg = rx.await.unwrap();
debug!(msg, "message received");
```

It's very short. We emit a debug message, await on the oneshot receiver and when we eventually get a message we emit another debug message with the contents of the message.

We expect (although there may not be a guarantee) that the `receiver` task will be polled first, and the traces show that this is indeed the case. Let's see how far we get with our receiver task.

{% traces() %}<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.587997Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>5</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=receiver, task.id=3}</span>  <b><u><span style='color:#5aba84'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.588051Z</span> <span style='color:#5c8dce'>DEBUG</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>5</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=receiver, task.id=3}</span> 
     <span style='color:#aaa'>⤷</span> <b><span style='color:#aaa'>tokio_oneshot</span></b>: <span style='color:#aaa'>awaiting message</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.588105Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span>  <b><u><span style='color:#df5853'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.588166Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#5c8dce'>⤷</span> <span style='color:#5c8dce'>runtime.resource.async_op[<b><span style='color:#508ee3'>3</span></b></span><span style='color:#5c8dce'>]{source=&quot;Receiver::await&quot;}</span>  <b><u><span style='color:#508ee3'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.588219Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#5c8dce'>⤷</span> <span style='color:#5c8dce'>runtime.resource.async_op[<b><span style='color:#508ee3'>3</span></b></span><span style='color:#5c8dce'>]{source=&quot;Receiver::await&quot;}</span> 
       <span style='color:#e5e44d'>⤷</span> <span style='color:#e5e44d'>runtime.resource.async_op.poll[<b><span style='color:#f5f466'>4</span></b></span><span style='color:#e5e44d'>]{}</span>  <b><u><span style='color:#f5f466'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.588278Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#5c8dce'>⤷</span> <span style='color:#5c8dce'>runtime.resource.async_op[<b><span style='color:#508ee3'>3</span></b></span><span style='color:#5c8dce'>]{source=&quot;Receiver::await&quot;}</span> 
       <span style='color:#e5e44d'>⤷</span> <span style='color:#e5e44d'>runtime.resource.async_op.poll[<b><span style='color:#f5f466'>4</span></b></span><span style='color:#e5e44d'>]{}</span> 
         <span style='color:#9d4edd'>⤷</span> <b><span style='color:#c77dff'>tokio::task::waker</span></b>: <span style='color:#9d4edd'>op=&quot;waker.clone&quot;, task.id=5</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.588346Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#5c8dce'>⤷</span> <span style='color:#5c8dce'>runtime.resource.async_op[<b><span style='color:#508ee3'>3</span></b></span><span style='color:#5c8dce'>]{source=&quot;Receiver::await&quot;}</span> 
       <span style='color:#e5e44d'>⤷</span> <span style='color:#e5e44d'>runtime.resource.async_op.poll[<b><span style='color:#f5f466'>4</span></b></span><span style='color:#e5e44d'>]{}</span> 
         <span style='color:#ffbf69'>⤷</span> <b><span style='color:#ff9f1c'>runtime::resource::poll_op</span></b>: <span style='color:#ffbf69'>op_name=&quot;poll_recv&quot;, is_ready=false</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.588415Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#5c8dce'>⤷</span> <span style='color:#5c8dce'>runtime.resource.async_op[<b><span style='color:#508ee3'>3</span></b></span><span style='color:#5c8dce'>]{source=&quot;Receiver::await&quot;}</span> 
       <span style='color:#e5e44d'>⤷</span> <span style='color:#e5e44d'>runtime.resource.async_op.poll[<b><span style='color:#f5f466'>4</span></b></span><span style='color:#e5e44d'>]{}</span>  <b><u><span style='color:#f5f466'>exit</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.588472Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#5c8dce'>⤷</span> <span style='color:#5c8dce'>runtime.resource.async_op[<b><span style='color:#508ee3'>3</span></b></span><span style='color:#5c8dce'>]{source=&quot;Receiver::await&quot;}</span>  <b><u><span style='color:#508ee3'>exit</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.588524Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span>  <b><u><span style='color:#df5853'>exit</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.588571Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>5</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=receiver, task.id=3}</span>  <b><u><span style='color:#5aba84'>exit</span></u></b>
{% end %}

In these traces we can see that we enter the `runtime.spawn` span for our `receiver` task and emit the debug event "awaiting message".

Then we enter each of the 3 nested spans, which were created earlier, in turn:
- <span style='color:#ba5a57;background-color:#2b303b;padding:2px'><code>runtime.resource</code></span>
- <span style='color:#5c8dce;background-color:#2b303b;padding:2px'><code>runtime.resource.async_op</code></span> `source=Receiver::await`
- <span style='color:#e5e44d;background-color:#2b303b;padding:2px'><code>runtime.resource.async_op.poll</code></span>

Once we're inside our span hierarchy 3 levels deep, we clone the waker for the current task (the `receiver` task). Afterwards we see a new instrumentation event `runtime::resource::poll_op`. This event comes with 2 fields: `op_name=poll_recv` and `is_ready=false`. This event is emitted after `poll_recv` has been called on the oneshot channel's internal state and indicates that it has returned `Poll::Pending` and so it isn't ready yet.

After that our 3 levels of spans exit in the opposite order to which they entered. As a small aside: this enter-exit stack behaviour isn't actually guaranteed by tracing, the spans could have exited in any order.

Since our task has awaited on the oneshot receiver, it will also yield back to the runtime, and the `runtime.spawn` span exits, which is where this snippet of traces ends.

### `async_op` vs. `async_op.poll`

One question you may be asking yourself at this point is what the difference is between a `runtime.resource.async_op` span and a `runtime.resource.async_op.poll` span.

I'm still not entirely sure, but from reading through comments in the 2 Tokio PRs, it seems to be that a `runtime.resource.async_op` span represents a future and it's lifetime and a `runtime.resource.async_op.poll` span only represents the poll operations on that future.

This makes some sense, but I'm not sure why we need the extra `runtime.resource.async_op.poll` span. A `runtime.spawn` span represents the future that is the basis for a task, I would think that `runtime.resource.async_op` could be the same. Currently the `runtime.resource.async_op` span is entered more times than the future is polled, but that is mostly so that specific events can be emitted within the right scope. I wonder whether we could use explicit parents to link those events to the necessary parent and then reserve entering the `runtime.resource.async_op` span for when a task is polled. We could set it up so that the state update belongs to an async op (`ready=true|false`) and simplify the instrumentation somewhat.

Having a separate `runtime.resource.async_op.poll` may still be necessary if we think that a future could be moved between tasks between polls (although I'm not sure this could be managed, it's certainly fraught with problems), but currenlty there isn't a new `runtime.resource.async_op.poll` span for each poll anyway, there is a one-to-one relationship between `runtime.resource.async_op` and `runtime.resource.async_op.poll` spans.

## oneshot sender task

The `sender` task also has only a small bit of code.

```rust
debug!("sending message");
tx.send(5).unwrap();
debug!("message sent");
```

Here we emit an initial debug message `"sending message"`, then we send our message `5` via the oneshot sender that we have. We unwrap the result as we expect it to pass (and would really rather just fail than gracefully handle something unexpected in demo code). Finally we emit another debug message `"message sent"`.

An important point to note here is that the `send` method on the oneshot sender isn't async. It doesn't block either. It will either send the first and only message through the channel, or it will fail.

Let's have a look at the traces for this part of the code.

{% traces() %}<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.588625Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>6</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=sender, task.id=4}</span>  <b><u><span style='color:#5aba84'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.588674Z</span> <span style='color:#5c8dce'>DEBUG</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>6</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=sender, task.id=4}</span> 
     <span style='color:#aaa'>⤷</span> <b><span style='color:#aaa'>tokio_oneshot</span></b>: <span style='color:#aaa'>sending message</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.588726Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>6</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=sender, task.id=4}</span> 
     <span style='color:#9d4edd'>⤷</span> <b><span style='color:#c77dff'>tokio::task::waker</span></b>: <span style='color:#9d4edd'>op=&quot;waker.wake_by_ref&quot;, task.id=5</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.588788Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span>  <b><u><span style='color:#df5853'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.588836Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#c9184a'>⤷</span> <b><span style='color:#ff4d6d'>runtime::resource::state_update</span></b>: <span style='color:#c9184a'>value_sent=true, value_sent.op=&quot;override&quot;</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.588893Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span>  <b><u><span style='color:#df5853'>exit</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.588941Z</span> <span style='color:#5c8dce'>DEBUG</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>6</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=sender, task.id=4}</span> 
     <span style='color:#aaa'>⤷</span> <b><span style='color:#aaa'>tokio_oneshot</span></b>: <span style='color:#aaa'>message sent</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.588991Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>6</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=sender, task.id=4}</span>  <b><u><span style='color:#5aba84'>exit</span></u></b>
{% end %}

This snippet is a bit shorter than for the first time the receiver was polled. Let's go through it.

We poll the `sender` task, which we see as entering the corresponding `runtime.spawn` span. Then we see our first debug message. Interestingly, we then wake the waker for the `receiver` task (we know which one it is because the `task.id` field matches the span ID of the `receiver` task's `runtime.spawn` span). It's only after the waker is woken that we enter the `runtime.resource` span and update the oneshot channel's resource state with a `runtime::resource::state_update` event. We set `value_sent=true`, so we know that by this point, the value has been sent (it is set in the channel's internal state).

I think that perhaps this could be reworked to enter the `runtime.resource` span for the duration of the `send` function, that way we would be able to link waking the waker to not just this task (the `sender` task), but also to this specific `runtime.resource` which represents the oneshot channel.

Another interesting thing is that the waker is woken by reference. Normally when a resource clones a waker it will wake it by value (which consumes the waker in the process). I was wondering why this isn't the case here, so I went to where the waker is woken by reference in the code ([`oneshot.rs:1133`](https://github.com/tokio-rs/tokio/blob/tokio-1.39.1/tokio/src/sync/oneshot.rs#L1133)) and found this:

```rust
// TODO: Consume waker?
```

That line was last modified in 2019 ([tokio-rs/tokio#1120](https://github.com/tokio-rs/tokio/pull/1120)). This is fine though, when the channel is dropped, the waker will be as well.

Finally, we see our second debug message `"message sent"` and then the `runtime.spawn` span ends, indicating that the task has yielded to the runtime. Since we have no `.await` points in this task, we know that the task must have returned `Poll::Ready`, which we can confirm in the next 3 traces.

{% traces() %}<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.589041Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>6</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=sender, task.id=4}</span>  <b><u><span style='color:#5aba84'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.589088Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>6</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=sender, task.id=4}</span>  <b><u><span style='color:#5aba84'>exit</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.589133Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>6</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=sender, task.id=4}</span>  <b><u><span style='color:#5aba84'>close</span></u></b>
{% end %}

Here we see the `runtime.spawn` span quickly enter and exit, before finally closing.

### enter-exit dance

This last enter-exit pair before a `runtime.spawn` span closes doesn't represent a poll, it is done by the [`Instrumented`](https://docs.rs/tracing/0.1.40/tracing/instrument/struct.Instrumented.html) struct when it gets dropped. It [enters the span](https://github.com/tokio-rs/tracing/blob/tracing-0.1.40/tracing/src/instrument.rs#L277) that is instrumenting the future prior to dropping it.

This behaviour was introduced in [tokio-rs/tracing#2541](https://github.com/tokio-rs/tracing/issues/2541) specifically as a way to have the instrumenting span in context for the drop operation (it was then fixed, because the original implementation caused an [unintended breaking change](https://github.com/tokio-rs/tracing/issues/2578) which is now on [Predrag](https://predr.ag/)'s [radar](https://github.com/obi1kenobi/cargo-semver-checks/issues/5), but no cargo-semver-checks lint has been implemented yet). Unfortunately this also means that we have to take into consideration that a `runtime.spawn` span will always have 1 extra enter-exit pair than actual polls. The only alternative would be to vendor the `Instrumented` struct without the drop behaviour, which seems excessive.

Now that the `sender` task has completed, we can go back to our `receiver` task which should now get polled again!

## receiving the message

As we were hoping, now that the `sender` has sent a message through the oneshot channel, the receiver gets polled.

The next swath of traces is a bit long, so let's just look up until the `runtime.resource.async_op` span enters and exits again.

{% traces() %}   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>5</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=receiver, task.id=3}</span>  <b><u><span style='color:#5aba84'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.589248Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span>  <b><u><span style='color:#df5853'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.589294Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#5c8dce'>⤷</span> <span style='color:#5c8dce'>runtime.resource.async_op[<b><span style='color:#508ee3'>3</span></b></span><span style='color:#5c8dce'>]{source=&quot;Receiver::await&quot;}</span>  <b><u><span style='color:#508ee3'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.589345Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#5c8dce'>⤷</span> <span style='color:#5c8dce'>runtime.resource.async_op[<b><span style='color:#508ee3'>3</span></b></span><span style='color:#5c8dce'>]{source=&quot;Receiver::await&quot;}</span> 
       <span style='color:#e5e44d'>⤷</span> <span style='color:#e5e44d'>runtime.resource.async_op.poll[<b><span style='color:#f5f466'>4</span></b></span><span style='color:#e5e44d'>]{}</span>  <b><u><span style='color:#f5f466'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.589402Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#5c8dce'>⤷</span> <span style='color:#5c8dce'>runtime.resource.async_op[<b><span style='color:#508ee3'>3</span></b></span><span style='color:#5c8dce'>]{source=&quot;Receiver::await&quot;}</span> 
       <span style='color:#e5e44d'>⤷</span> <span style='color:#e5e44d'>runtime.resource.async_op.poll[<b><span style='color:#f5f466'>4</span></b></span><span style='color:#e5e44d'>]{}</span> 
         <span style='color:#ffbf69'>⤷</span> <b><span style='color:#ff9f1c'>runtime::resource::poll_op</span></b>: <span style='color:#ffbf69'>op_name=&quot;poll_recv&quot;, is_ready=true</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.589470Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#5c8dce'>⤷</span> <span style='color:#5c8dce'>runtime.resource.async_op[<b><span style='color:#508ee3'>3</span></b></span><span style='color:#5c8dce'>]{source=&quot;Receiver::await&quot;}</span> 
       <span style='color:#e5e44d'>⤷</span> <span style='color:#e5e44d'>runtime.resource.async_op.poll[<b><span style='color:#f5f466'>4</span></b></span><span style='color:#e5e44d'>]{}</span> 
         <span style='color:#9d4edd'>⤷</span> <b><span style='color:#c77dff'>tokio::task::waker</span></b>: <span style='color:#9d4edd'>op=&quot;waker.drop&quot;, task.id=5</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.589539Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#5c8dce'>⤷</span> <span style='color:#5c8dce'>runtime.resource.async_op[<b><span style='color:#508ee3'>3</span></b></span><span style='color:#5c8dce'>]{source=&quot;Receiver::await&quot;}</span> 
       <span style='color:#e5e44d'>⤷</span> <span style='color:#e5e44d'>runtime.resource.async_op.poll[<b><span style='color:#f5f466'>4</span></b></span><span style='color:#e5e44d'>]{}</span>  <b><u><span style='color:#f5f466'>exit</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.589597Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#5c8dce'>⤷</span> <span style='color:#5c8dce'>runtime.resource.async_op[<b><span style='color:#508ee3'>3</span></b></span><span style='color:#5c8dce'>]{source=&quot;Receiver::await&quot;}</span>  <b><u><span style='color:#508ee3'>exit</span></u></b>

{% end %}

Here we see the `runtime.spawn` span representing the `receiver` task enter, which is expected - it just got polled again after having been woken by the `send` operation.

Then successively the `runtime.resource`, `runtime.resource.async_op`, and `runtime.resource.async_op.poll` spans are all entered. This indicates that the receiver is being polled. We see a `runtime::resource::poll_op` event indicating that the leaf future was polled via `poll_recv` and that it returned `Poll::Ready` (because that event has the field `is_ready=true`). So it looks like we've successfully got a result. Note that at this point, we don't actually know whether we have a value or whether an error will be returned.

Following the completion of the poll operation, we see that the waker is dropped. This is expected as the sender half of the oneshot channel woke the `receiver` task by reference, not by value, which we saw in the [oneshot sender task](#oneshot-sender-task) section.

After that, the `runtime::resource::poll_op` exits and then so does the `runtime.resource.async_op` span.

Now let's follow the traces through until the `receiver` task completes.

{% traces() %}<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.589649Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span>  <b><u><span style='color:#df5853'>exit</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.589843Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#5c8dce'>⤷</span> <span style='color:#5c8dce'>runtime.resource.async_op[<b><span style='color:#508ee3'>3</span></b></span><span style='color:#5c8dce'>]{source=&quot;Receiver::await&quot;}</span> 
       <span style='color:#e5e44d'>⤷</span> <span style='color:#e5e44d'>runtime.resource.async_op.poll[<b><span style='color:#f5f466'>4</span></b></span><span style='color:#e5e44d'>]{}</span>  <b><u><span style='color:#f5f466'>close</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.589900Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span> 
     <span style='color:#5c8dce'>⤷</span> <span style='color:#5c8dce'>runtime.resource.async_op[<b><span style='color:#508ee3'>3</span></b></span><span style='color:#5c8dce'>]{source=&quot;Receiver::await&quot;}</span>  <b><u><span style='color:#508ee3'>close</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.589955Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ba5a57'>⤷</span> <span style='color:#ba5a57'>runtime.resource[<b><span style='color:#df5853'>2</span></b></span><span style='color:#ba5a57'>]{concrete_type=&quot;Sender|Receiver&quot;, kind=&quot;Sync&quot;}</span>  <b><u><span style='color:#df5853'>close</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.590014Z</span> <span style='color:#5c8dce'>DEBUG</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>5</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=receiver, task.id=3}</span> 
     <span style='color:#aaa'>⤷</span> <b><span style='color:#aaa'>tokio_oneshot</span></b>: <span style='color:#aaa'>msg=5 message received</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.590070Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>5</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=receiver, task.id=3}</span>  <b><u><span style='color:#5aba84'>exit</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.590207Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>5</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=receiver, task.id=3}</span>  <b><u><span style='color:#5aba84'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.590257Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>5</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=receiver, task.id=3}</span>  <b><u><span style='color:#5aba84'>exit</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.590433Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>5</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=receiver, task.id=3}</span>  <b><u><span style='color:#5aba84'>close</span></u></b>
{% end %}

First up, we see the `runtime.resource` span exit (it had entered in the first traces we saw in this section). After that, our hierarchy of three spans close in the reverse order to their creation: `runtime.resource.async_op.poll`, `runtime.resource.async_op`, and finally `runtime.resource`. We now understand that the oneshot channel itself has been dropped.

The oneshot channel drops before the end of the scope because awaiting on the receiving half consumes it, so once the future completes that receiving half is dropped (and the sender half had already been dropped earlier).

Finally, we see our debug event stating that the message has been received, and giving us the value (`5`!). Now we know that receiving the message was successful.

After our debug event, the `runtime.spawn` span representing the `receiver` task exits (complete with [enter-exit dance](#enter-exit-dance)) and then gets dropped.

And that's the end of the resource instrumentation. Or at least, it should be.

## resource state values

Perhaps you remember that when the resource was created, there were 4 resource state values which were set: 
- `tx_dropped=false`
- `rx_dropped=false`
- `value_sent=false`
- `value_received=false`

However, only 1 of these was updated! That was:
- `value_sent=true`

What happened to the others? I did a bit of digging into the implementation, and found that there are event macros that update these values in the code, but it looks like they're not hit on all code paths. It just so happens that my test code missed a bunch of them. We already have some [tests for the tracing instrumentation](https://github.com/tokio-rs/tokio/tree/tokio-1.39.1/tokio/tests/tracing-instrumentation) in Tokio. However, they're currently quite limited.

The plan is to extend them to cover more cases in the future, it looks like I've already found the first thing that needs a test!

## finishing up

We've created a oneshot channel, waiting for a message, sent that message, and then finally received it. And we got to see all of this happening through the metaphorical eyes of Tokio's tracing instrumentation.

Let's remind ourselves of the final bit of code.

```rust
async fn tokio_oneshot() {
    // Everything else that has already happened.

    debug!(?jh, "awaiting receiver task");
    jh.await.unwrap();
    debug!("good-bye");
}
```

We had already seen the debug event `"awaiting receiver task"` earlier on. Since this task ran until we awaited on the join handle for the `receiver` task (the variable `jh`). Here are the final traces.

{% traces() %}<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.590493Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#9d4edd'>⤷</span> <b><span style='color:#c77dff'>tokio::task::waker</span></b>: <span style='color:#9d4edd'>op=&quot;waker.wake_by_ref&quot;, task.id=1</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.590554Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=tokio-oneshot, task.id=2}</span>  <b><u><span style='color:#5aba84'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.590614Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=tokio-oneshot, task.id=2}</span> 
     <span style='color:#9d4edd'>⤷</span> <b><span style='color:#c77dff'>tokio::task::waker</span></b>: <span style='color:#9d4edd'>op=&quot;waker.drop&quot;, task.id=1</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.590674Z</span> <span style='color:#5c8dce'>DEBUG</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=tokio-oneshot, task.id=2}</span> 
     <span style='color:#aaa'>⤷</span> <b><span style='color:#aaa'>tokio_oneshot</span></b>: <span style='color:#aaa'>good-bye</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.590727Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=tokio-oneshot, task.id=2}</span>  <b><u><span style='color:#5aba84'>exit</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.590774Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=tokio-oneshot, task.id=2}</span>  <b><u><span style='color:#5aba84'>enter</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.590820Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=tokio-oneshot, task.id=2}</span>  <b><u><span style='color:#5aba84'>exit</span></u></b>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.590865Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#489e6c'>⤷</span> <span style='color:#489e6c'>runtime.spawn[<b><span style='color:#5aba84'>1</span></b></span><span style='color:#489e6c'>]{kind=task, task.name=tokio-oneshot, task.id=2}</span>  <b><u><span style='color:#5aba84'>close</span></u></b>
{% end %}

Now that we've remembered that our `tokio_oneshot` task had awaited the join handle for the `receiver` task, it's no surprise that the first event we see is a waker event, specifically the `tokio_oneshot` task getting woken up. This `tokio::task::waker` event has no parent span, so it didn't occur in any task or resource that we're aware of. This is happening somewhere inside the runtime.

After that we see the `runtime.spawn` span representing the `tokio_oneshot` task is entered. The waker is dropped, presumably when the join handle is consumed, although to be sure we'd need to instrument join handles as described in the section [join handles are resources too](#join-handles-are-resources-too).

To finish off, we see our debug message `"good-bye"` and then the `runtime.spawn` span exits and closes ([enter-exit dance](#enter-exit-dance) included).

That's almost the end of the traces, but it isn't...

## but wait there's more

There are actually 3 more traces at the end, and it took me a little while to realise what was going on. Here they are.

{% traces() %}<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.590983Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#c9184a'>⤷</span> <b><span style='color:#ff4d6d'>runtime::resource::state_update</span></b>: <span style='color:#c9184a'>tx_dropped=true, tx_dropped.op=&quot;override&quot;</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.591050Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#ffbf69'>⤷</span> <b><span style='color:#ff9f1c'>runtime::resource::poll_op</span></b>: <span style='color:#ffbf69'>op_name=&quot;poll_recv&quot;, is_ready=true</span>
<span style='opacity:0.67'><b><span style='color:#aaa'>2024-07-31</span></b></span><span style='opacity:0.67'>T<b><span style='color:#aaa'>15:35:48</span></b></span><span style='opacity:0.67'>.591106Z</span> <span style='color:#9d4edd'>TRACE</span> 
   <span style='color:#c9184a'>⤷</span> <b><span style='color:#ff4d6d'>runtime::resource::state_update</span></b>: <span style='color:#c9184a'>rx_dropped=true, rx_dropped.op=&quot;override&quot;</span>
{% end %}

These traces show state updates that look like ones we've seen previously. They don't have any parent spans, which seems weird, but we can see the following things happen:
- a sender is dropped
- a receiver is polled and returns `Poll::Ready`
- a receiver is dropped

We've seen 2 of these messages before: the sender getting dropped and the `poll_recv` operation. And believe it or not, these 3 traces just happen to belong to the same resource that we've been investigating during this whole blog post, they're from a oneshot channel! This can be verified with a quick search for `rx_dropped` in the Tokio codebase. No link for this as only the default branch for a project is indexed, so the link may be out of date if more channels get instrumented.

So what is it? It turns out that the mechanism used to [shut down the blocking pool](https://github.com/tokio-rs/tokio/blob/tokio-1.39.1/tokio/src/runtime/blocking/shutdown.rs) uses a oneshot channel internally, so these messages come from there!

With that mystery solved, we've made our way to the end of the traces and the end of the post. I learnt a lot while writing this post, which was pretty much the idea. I hope that the 3 of you who have an interest in the internals of Tokio's tracing instrumentation also learnt something!