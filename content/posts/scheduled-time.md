+++
title = "task scheduled time in console"
slug = "task-scheduled-time-in-console"
author = "hds"
date = "2023-04-26"
+++

> Update (2023-05-10): A new version of `tokio-console` is [available](#availability) including this change!

Last week we merged a small set of cool changes in [Tokio Console](https://github.com/tokio-rs/console).

They added support for tracking and displaying per task scheduled time.

For the impatient amongst you, here's a screenshot.

![Screenshot of `tokio-console` displaying the task details for the `burn` task in the `app` example.](/img/scheduled-time/app_example-burn_task_details.png)

But what is a task's scheduled time?

Actually, let's first make sure we're all on the same page.

What's a task?

### tasks

We're discussing async Rust here.

So when I say task, I'm talking about it in that context.

From the Tokio documentation for [`tokio::task`](https://docs.rs/tokio/1.27.0/tokio/task/):

> A *task* is a light weight, non-blocking unit of execution.

A task is like a thread, but lighter-weight.

The asynchronous runtime (e.g. Tokio) is responsible for scheduling tasks across its workers.

We won't go much more into detail than that for now.

### scheduled time

We often think of a task as having 2 states.

**Busy**: when it's being executed.

**Idle**: when it's waiting to be executed.

Let's look at an example of a task moving between these two states.

![Time-status diagram showing 1 task in one of 2 states: idle, busy.](/img/scheduled-time/scheduled_time-example_busy_idle.png)

We see that the task is either idle or busy.

When a task stops doing work, it yields to the runtime.

This is usually because it is `await`ing some other future.

Although it could also voluntarily yield.

(in Tokio this is done by calling `tokio::task::yield_now().await`)

When a task yields to the runtime, it needs to be woken up.

Tasks get woken by a waker.

(tautologies galore)

We're not going to get into the mechanics of wakers today.

Enough to know that when a task is woken, it is ready to work.

But the runtime might not have a worker to place it on.

So there could be some delay between when a task is woken and when it becomes busy.

This is the scheduled time.

![Time-status diagram showing 1 task in one of 3 states: idle, scheduled, busy.](/img/scheduled-time/scheduled_time-example_scheduled_busy_idle.png)

Why is this interesting?

Let's have a look

### scheduling delays

Let's look at a case with 2 tasks.

To make things simple, we'll suppose a runtime with only 1 worker.

This means that only a single task can be busy at a time.

Here's a time-status diagram of those's 2 tasks.

![Time-status diagram showing 2 tasks, each in one of 2 states: idle, busy. There is no point at where both tasks are busy at the same time.](/img/scheduled-time/scheduled_time_2_tasks-busy_idle.png)

Nothing looks especially wrong.

(there is one thing, but we don't want to get ahead of ourselves).

But perhaps the behavior isn't everything we want it to be.

Perhaps Task 2 is sometimes taking a long time to respond to messages from a channel.

Why is this?

We don't see it busy for long periods.

Let's include the scheduled time.

![Time-status diagram showing 2 tasks, each in one of 3 states: idle, scheduled, busy. There is no point at where both tasks are busy at the same time. There is one moment when task 1 is busy for a long time and during part of that time, task 2 is scheduled for longer than usual.](/img/scheduled-time/scheduled_time_2_tasks-scheduled_busy_idle.png)

Now something does jump out at us.

While task 1 is busy, task 2 is scheduled for a lot longer than usual.

That's something to investigate.

It also makes is clear that task 1 is blocking the executor.

That means that it's busy for so long that it doesn't allow other tasks to proceed.

Bad task 1.

That's the thing that a trained eye might have caught before.

But we don't all benefit from trained eyes.

### scheduled time in the console

Tokio console doesn't have these pretty time-status diagrams.

Yet, at least.

But you can now see the scheduled time of each task.

![Tokio console showing the task list view. There is a column labelled Sched for the scheduled time.](/img/scheduled-time/app_example-task_list.png)

And sort by that column too.

Let's look at the task with the highest scheduled time, `task2`.

![Tokio console showing the task detail view. There are 2 sets of percentiles and histograms. The top one is for poll (busy) times, the bottom one is for scheduled times.](/img/scheduled-time/app_example-task2_details.png)

It's quickly clear that `task2` spends most of its time "scheduled".

Exactly 61.34% of its time when this screenshot was taken.

We can also see that during most poll cycles, `task2` spends more than 1 second scheduled.

And at least once, over **17 seconds**!

How about we have a look at a more common scheduled times histogram.

Let's look at the task details for the `burn` task that we saw at the beginning.

![Tokio console showing the task detail view for the `burn` task. The scheduled times histogram is more as we'd expect, clustered around the lower end.](/img/scheduled-time/app_example-burn_task_details.png)

Here we see that the scheduled times are more reasonable.

Between 22 and 344 microseconds.

(by the way, this example app is available in the [console repo](https://github.com/tokio-rs/console/blob/main/console-subscriber/examples/app.rs))

Of course, maybe 17 seconds is just fine in your use case.

But with Tokio console, you now have that information easily available.

### availability

(updated 2023-05-10)

The scheduled time feature has released!

To use it, you need at least [`tokio-console` 0.1.8](https://crates.io/crates/tokio-console/0.1.8) and [`console-subscriber` 0.1.9](https://crates.io/crates/console-subscriber/0.1.9).