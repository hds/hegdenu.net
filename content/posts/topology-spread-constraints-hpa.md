+++
title = "topology spread constraints and horizontal pod autoscaling"
slug = "topology-spread-constraints-and-horizontal-pod-autoscaling"
author = "hds"
date = "2023-03-20"
+++

I'm back with more gotchas involving [topology spread constraints](https://kubernetes.io/docs/concepts/scheduling-eviction/topology-spread-constraints/).

This post is effectively part 2 of something that was never intended to be a series.

Part 1 explored [topology spread constraints and blue/green deployments](@/posts/topology-spread-constraints.md).

To be really honest, both these posts should include an extra quantifiers:

"in kubernetes on statefulsets with attached persistent volumes."

But that makes the title too long, even for me.

What's wrong this time?

### the setup

Let's review the setup, in case you haven't read part 1.

(it's recommended, but not required reading.)

(but if you don't know what a topology spread constraint is, then maybe read it.)

(or at least read [aside: topology spread constraints](@/posts/topology-spread-constraints.md#aside-topology-spread-constraints).)

The app in question is powered by a statefulset.

Each pod has a persistent volume attached.

(hence the statefulset.)

This time we're talking about production.

In production, we scale the number of pods based on the CPU usage of our web server container.

This is known as [horizontal pod autoscaling](https://kubernetes.io/docs/tasks/run-application/horizontal-pod-autoscale/).

### aside: horizontal pod autoscaling

We're going to use kubernetes terminology.

In kubernetes a [pod](https://kubernetes.io/es/docs/concepts/workloads/pods/pod/) is the minimum scaling factor.

If you're coming from standard cloud world, it's sort of equivalent to an instance.

(but with containers and stuff.)

If you're coming from a data center world (on- or off-site) then it's a server.

(but with virtualization and stuff.)

Traditionally you can scale an application in 2 directions.

Vertically and horizontally.

Vertical scaling means making the pods bigger.

More CPU.

More Memory.

That sort of thing.

Horizontal scaling means adding more pods.

Kubernetes has built in primitives for horizontal pod autoscaling.

The idea is that you measure some metric from each pod, averaged across all pods.

(or more than one, but we won't go into that.)

And then you give Kubernetes a target value for that metric.

Kubernetes will then add and remove pods (according to yet more rules) to meet the target value.

Let's imagine we are scaling on pod CPU.

We have two pods and together their average CPU is well over the target.

![Two pods shown with their CPU usage as a bar graph. The target value is shown as a horizontal line. The average of the two pods is shown as another horizontal line which is significantly above the target line.](/img/topology-spread-constraints-hpa/hpa-1.png)

As the average is significantly above the target, Kubernetes will scale up.

Now we have 3 pods, each with less CPU than before.

![Three pods shown with their CPU usage as a bar graph. The target value is shown as a horizontal line. The average of the three pods is shown as another horizontal line which is now below the target line.](/img/topology-spread-constraints-hpa/hpa-2.png)

The average of the CPU usage of the 3 pods is now a little less than the target.

This is enough to make the horizontal pod autoscaler happy.

Of course, all this assumes that your CPU scales per request and that more pods means fewer requests per pod.

Otherwise, this is a poor metric to scale on.

### the problem

We previously had issues with pods in pending state for a long time (see [part 1](@/posts/topology-spread-constraints.md)).

So we added monitoring for that!

Specifically an alarm when a pod had been in pending state for 30 minutes or more.

This is much longer than it should be for pods that start up in 8 - 10 minutes.

The new alert got deployed on Friday afternoon.

(Yes, we deploy on Fridays, it's just a day.)

And then the alert was triggering **all weekend**.

First reaction was to ask how we could improve the alert.

Then we looked into the panel driving the alerts.

There were pods in pending state for 40 minutes.

50 minutes.

**65 minutes!**

What was going on?

### the investigation

Looking at the metrics, we saw a pattern emerging.

A single pod was in pending state for a long period.

Then another pod went into pending state.

Shortly afterwards, both pods were ready.

It looked like the following.

![A time/status chart showing pending and ready states (or inactive) for 4 pods. The first 2 pods are ready for the whole time period. The 3rd pod goes into pending for a long time. Then the 4th pod goes into pending, followed shortly by the 3rd pod becoming ready and then the 4th pod becoming ready as well.](/img/topology-spread-constraints-hpa/pending-alerts.png)

(Actually the panel didn't look like that at all.)

(What we have available is somewhat more obtuse.)

(But that is how I wish the panel looked.)

This same pattern appeared several times in the space of a few hours.

Oh right, of course.

It's the topology spread constraints again.

It all has to do with how a `statefulset` is scheduled.

### aside: pod management policy

The [pod management policy](https://kubernetes.io/docs/tutorials/stateful-application/basic-stateful-set/#pod-management-policy) determines how statefulset pods are scheduled.

It has 2 possible values.

The default is `OrderedReady`.

This means that each pod waits for all previous pods to be scheduled before it gets scheduled.

![A time/status chart showing pending and ready states for 4 pods, numbered 0 to 3. Pod-0 starts in pending and then moves to ready. Each subsequent pod starts pending when the preceding pod is ready, then moves to ready itself some time later.](/img/topology-spread-constraints-hpa/pmp-ordered-ready.png)


The other options is `Parallel`.

In this case, pods are all scheduled together.

![A time/status chart showing pending and ready states for 4 pods, numbered 0 to 3. All pods start in pending and then move to ready at more or less the same time.](/img/topology-spread-constraints-hpa/pmp-parallel.png)

That's like a [deployment](https://kubernetes.io/docs/concepts/workloads/controllers/deployment/).

Some applications require the ordering guarantees of `OrderedReady`.

However, it makes deployment `N` times slower if you have `N` pods.

We have no need of those ordering guarantees.

So we use the `Parallel` pod management policy.

### the answer

Now that we have all the context, let's look at what happens.

Let's start a new `statefulset` with 4 pods.

We set the maximum skew on our zone topology spread constraints to 1.

At most we can have a difference of 1 between the number of pods in zones.

Our 4 pods are distributed as evenly as possible across our 3 zones.

![Three zones containing 4 pods, each pod has a volume with the same index as the pod. Zone A contains pod-0/vol-0 and pod-2/vol-2. Zone B contains pod-1/vol-1. Zone C contains pod-3/vol-3.](/img/topology-spread-constraints-hpa/tsc-sts-scale-1.png)

So far, so good.

We don't have so much load right now, so let's scale down to 2 pods.

![Three zones containing 2 pods, each pod has a volume with the same index as the pod. There are 2 additional volumes without pods. Zone A contains pod-0/vol-0 and vol-2 without a pod. Zone B contains pod-1/vol-2. Zone C contains vol-3 without a pod.](/img/topology-spread-constraints-hpa/tsc-sts-scale-2.png)

After scaling down, we don't remove the volumes.

The volumes are relatively expensive to set up.

The 8 - 10 minutes that pods take to start is mostly preparing the volume.

Downloading data, warming up the on-disk cache.

A pod with a pre-existing volume starts in 1 - 2 minutes instead.

So it's important to keep those volumes.

Now let's suppose that the load on our stateful set has increased.

We need to add another pod.

So let's scale one up.

![Three zones containing 2 pods which are ready and 1 which is pending, each pod has a volume with the same index as the pod. There is 1 additional volumes without a pod. Zone A contains pod-0/vol-0 and pod-2/vol-2, but pod-2 can't be scheduled and is in pending state. Zone B contains pod-1/vol-2. Zone C contains vol-3 without a pod.](/img/topology-spread-constraints-hpa/tsc-sts-scale-3.png)

Now we hit our problem.

The next pod, `pod-2`, has a pre-existing volume in Zone A.

But if a new pod is scheduled in Zone A, we'll have 2 pods in Zone A.

And no pods in Zone C.

That's a skew of 2, greater than our maximum of 1.

So the scheduler basically waits for a miracle.

It knows that it can't schedule `pod-2` because it would breach the topology spread constraints.

And it can't schedule a pod in Zones B or C because `vol-2` can't be attached.

So it gives up and tries again later.

And fails.

And gives up and tries again later.

And fails.

And this would go on until the end of time.

Except just then, a miracle occurs.

The load on our stateful set increases even more.

We need another pod!

Now we can schedule `pod-3` in Zone C.

And schedule `pod-2` in Zone A.

(where we've been trying to schedule it for what feels like aeons.)

And our skew is 1!

![Three zones containing 2 pods which are ready and 2 which can now move to ready, each pod has a volume with the same index as the pod. Zone A contains pod-0/vol-0 and pod-2/vol-2, the latter has just been successfully scheduled. Zone B contains pod-1/vol-2. Zone C contains pod-3/vol-3 which has just been successfully scheduled.](/img/topology-spread-constraints-hpa/tsc-sts-scale-4.png)

And this is why we saw a single pod pending for a long time.

And then 2 pods go ready in quick succession.

### the solution

Unfortunately I don't have such a happy ending as in [part 1](@/posts/topology-spread-constraints.md).

The solution is to use the `OrderedReady` pod management policy.

But that's impractical due to the long per-pod start up time.

So the solution is to loosen the constraint.

Allow the topology spread constraints to be best effort, rather than strict.

### the ideal solution

Ideally, I want a **new** pod management policy.

I'd call it `OrderedScheduled`.

Each pod would be begin scheduling as soon as the previous pod was scheduled.

![A time/status chart showing pending and ready states for 4 pods, numbered 0 to 3. Pod-0 starts in pending and then moves to ready. Each subsequent pod starts pending shortly after the preceding pod starts pending, then moves to ready itself some time later. All the pods will coincide in pending state.](/img/topology-spread-constraints-hpa/pmp-ordered-scheduled.png)

That way, pods are scheduled in order.

So scaling up and down won't breach the topology spread constraints.

Of course, this is just an idea.

There are probably many sides that I haven't considered.




