+++
title = "testing instrumentation with tracing-mock"
slug = "tracing-mock-beta"
author = "hds"
date = "2024-12-11"
+++

If you're using Rust, you may have heard of the [`tracing`](https://docs.rs/tracing) crate. It provides enablers to emit spans and events from your code, which can then be picked up by tracing subscribers and used it all sorts of ways.

You might use this primarily for logging (instead of the older [`log`](https://docs.rs/log) crate), writing the traces out to `stdout` or a file. However, you can also push spans and events they contain to an open telemetry collector with [`tracing-opentelemetry`](https://docs.rs/tracing-opentelemetry), create flame graphs with [`tracing-flame`](https://docs.rs/tracing-flame), or a whole myriad [of other possibilities](https://github.com/tokio-rs/tracing#related-crates).

If you're using the tracing instrumentation in Tokio, then you'll be using [`console-subscriber`](https://docs.rs/console-subscriber) to pick up the traces, aggregate them, and then serve them over a gRPC channel to [`tokio-console`](https://docs.rs/tokio-console).

Now that you've got your library or application instrumented with `tracing`, you might want to write tests to ensure that you're emitting the right traces at the right times, and with the right values. Because what you don't want, is that you get an alert one day and only then discover that you're not actually recording the values you need to debug a problem in production, or that you have a span that is kept open forever because of some unforeseen child span.

This is where [`tracing-mock`](https://docs.rs/tracing-mock) comes in. It allows you to make assertions about what traces are emitted by your code.

I'm going to be referring to the traces in your code as instrumentation, because that's what they are. They are additional things added into your code base so that you can understand more about it as it runs.

## why would you test instrumentation?

Testing instrumentation is, unfortunately, not such a common practice.

(neither is testing bespoke test frameworks, but I'll leave that hill for another day)

However, like the other parts of your code that must meet functional and non-functional requirements to be fit for purpose, your instrumentation must also serve a purpose. Otherwise, what is it there for?

In fact, depending on how you make use of that instrumentation, it may be more important to test than "normal" code, since it could be less obvious that it is broken and less well understood that the rest of your code base.

Testing shows intent. A good test shows the way we want something to be, and communicates that this is a requirement of our code base, not a side effect. By testing the instrumentation in  your code, you're making clear how it should work, especially in the case that someone comes along and changes how it works by accident.

Finally, if you're in an environment where you're expected some arbitrary code coverage figure, recoverable error branches are often hard to test. By testing the instrumentation you stick in there (even if it's only a `DEBUG` level event), you can cleverly test the code path is taken without introducing additional mocking.

## introducing tracing-mock

Tracing has had a built in test library for a long time. It was originally a set of modules loaded from test files directly. Way back at the beginning of 2020 (almost 5 years ago at the time of writing), the issue [tokio-rs/tracing#539](https://github.com/tokio-rs/tracing/issues/539) was opened by [Jane] to factor out the testing support code into a separate crate and publish it to [crates.io](https://crates.io/).

The code did get factored out into a separate crate - `tracing-mock` in [tokio-rs/tracing#2009](https://github.com/tokio-rs/tracing/pull/2009) - in March 2022, but not much further work was done on getting it ready to be published. It probably didn't help that none of the functionality of the newly named `tracing-mock` crate was documented.

It was about 6 months later that I showed up trying to test that the spawn location was correct in the tracing instrumentation in Tokio. It was after fixing a case where the location wasn't correct due to a break in the `#[track_caller]` chain down to the creation of the task's span, as reported in [tokio-rs/tokio#5030](https://github.com/tokio-rs/tokio/issues/5030). After finding `tracing-mock` and trying it out, I realised that it worked just fine for that use case. I also realised that it not being on crates.io made using from Tokio's integration tests complicated (it basically required a separate crate outside of the workspace and with its own dedicated job on GitHub).

So I set about preparing `tracing-mock` for publication. Firstly documenting the entire public API and adding rustdoc tests. That solved 2 problems at once, it added plenty of examples to the API documentation and also added tests for much of the functionality. This crate may be one of the most extensively doc-tested crates on crates.io, most APIs contain a passing and a failing example which act as positive and negative tests.

Along the way there were a few things that needed to be fixed (that's what happens when you don't have tests), and some additions were needed to cover the sorts of use cases you run into when you want to test your code's instrumentation, instead of testing `tracing` itself - which was of course the whole point of the code that became `tracing-mock`.

Some [16 Pull Requests later](https://github.com/tokio-rs/tracing/pulls?q=is%3Apr+is%3Amerged+author%3Ahds+created%3A%3C%3D2024-11-29+mock+in%3Atitle+), we finally released [`tracing-mock` 0.1.0-beta.1](https://crates.io/crates/tracing-mock/0.1.0-beta.1) to crates.io last week!

## testing instrumentation

Let's write a somewhat contrived example of code we might want to test. A function that takes a `&str` and then emits a single tracing event using the passed `name` as a field and also includes the message `Hello!`.

```rust
pub fn hello(name: &str) {
    tracing::info!(name, "Hello!");
}
```

Now let's write a test for this. Don't be too shocked by the size of the tests, setting up and testing `tracing` can be a bit verbose.

```rust
use tracing::subscriber::with_default;
use tracing_mock::{expect, subscriber};

#[test]
fn hello_event() {
    let (subscriber, handle) =
        subscriber::mock()
            .event(expect::event().with_fields(
                expect::msg("Hello!").and(expect::field("name").with_value(&"Hayden")),
            ))
            .run_with_handle();

    with_default(subscriber, || {
        tracing_mock_beta::hello("Hayden");
    });

    handle.assert_finished();
}
```

Let's go through the test code.

First we set up our mock subscriber. This is the center of `tracing-mock`, it implements the [`Subscriber`](https://docs.rs/tracing/0.1/tracing/trait.Subscriber.html) trait from `tracing` so that it will receive spans and events emitted while it is active.

The [`MockSubscriber`](https://docs.rs/tracing-mock/0.1.0-beta.1/tracing_mock/subscriber/struct.MockSubscriber.html) works like a builder pattern, you successively call functions on it to set expectations. In this example, we set only a single expectation, that an event it emitted with two fields. Finally, we call `run_with_handle()` to get a `Subscriber` back together with a handle - we'll see what we use that for in the next paragraph.

We use `tracing`'s [`with_default()`](https://docs.rs/tracing/0.1/tracing/subscriber/fn.with_default.html) function to set our subscriber in the current thread for the scope of the lambda passed as the second argument. Within the lambda we call whatever we want to test from our code. Once `with_default()` has returned, we call `handle.assert_finished()` to assert the expectations we had provided earlier.

There are plenty of other things that can be asserted about an event, such as the level it is emitted at, the name, target, and more. There are details (and plenty of example) in the API documentation for [`ExpectedEvent`](https://docs.rs/tracing-mock/0.1.0-beta.1/tracing_mock/event/struct.ExpectedEvent.html).

### testing spans

Let's look at a short example testing spans. Spans have a lifetime where they are created, enter and exit - perhaps multiple times, and then get closed. The only point at which we can check many of a span's attributes is when it is created. After that, a `Subscriber` only receives the span's [`Id`](https://docs.rs/tracing/0.1/tracing/span/struct.Id.html) and while our mock subscriber does keep a mapping of `Id` to metadata, the metadata doesn't contain field values.

To show how this works, let's take a look at this function:

```rust
pub fn enter_exit() {
    let span = tracing::info_span!("best-span", anse);

    {
        let _guard = span.enter();
    }

    {
        let _guard = span.enter();
    }
}
```

It creates a span, then enters and exits the span twice. Finally the span will be closed when the `span` variable is dropped.

Let's test this span. The imports are the same as our previous test (and those are mostly the only imports you need from `tracing-mock`), so I'll skip them.

```rust
#[test]
fn hello_event() {
    let span = expect::span().named("life-universe-everything");
    let (subscriber, handle) = subscriber::mock()
        .new_span(
            span.clone()
                .with_fields(expect::field("answer").with_value(&42_u64)),
        )
        .enter(&span)
        .exit(&span)
        .enter(&span)
        .exit(&span)
        .drop_span(&span)
        .run_with_handle();

    with_default(subscriber, || {
        tracing_mock_beta::enter_exit(42);
    });

    handle.assert_finished();
}
```

In this test, we're going to create a variable for our expected span, as we'll be using it multiple times. TODO(hds): working here...