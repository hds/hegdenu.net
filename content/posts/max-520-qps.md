+++
title = "max 520 qps"
slug = "max-520-qps"
author = "hds"
date = "2026-05-31"
draft = true
+++

I had a fun riddle a couple of months ago, a webserver rewrite from Python to Rust that used half the CPU, but showed the same throughput in performance tests.

## the setup

We've got a Python webserver ([Tornado]) that wraps a C++ library that does pretty much all the work.

[Tornado]: https://www.tornadoweb.org

Python passes in validated request parameters and a pointer to a byte buffer, C++ looks up content in an in-memory index and then writes a protobuf response directly into the buffer. Python then returns the buffer contents as the response body.

## the original problem

This rewrite wasn't for the sake of a rewrite. We had some real performance issues in production.

We auto-scale this particular service based on CPU usage. We had to set the target fairly low, because we hadn't been able to achieve very good parallelism, but we had learned to live with this. However, after an increase in the volume of data in our in-memory index and therefore also in responses, we found that when we lost a replica (out of say, 6 replicas), we were seeing big latency spikes and even some requests timing out.

With 2 CPUs to serve requests, we had already set our auto-scaling target at 120%. To mitigate these new spikes, we dropped it down to 100%. The mitigation worked, but this was something that needed to be fixed!

Each of these replicas was still serving 500-600 QPS, so not a push over by any means, but we should be able to get more out of 2 CPUs.

## the prototype

This is the only bit of Python we have in our production stack, and I wasn't really sure how best to go about investigating why we couldn't get more CPU utilization.

Being Python, it could be a GIL (Global Interpreter Lock) problem, but I didn't want to go blaming it without some sort of proof. I looked into tools to profile the GIL, but I didn't find anything that looked like it would help me, not a Python performance expert, determine whether the GIL was really to blame or not.

So I did what every engineer who doesn't understand a system does, I rewrote it.

Well, I prototyped a rewrite, converting the Python layer (which isn't very large anyway) to Rust. I ended up playing around a bunch with possible optimizations, but the initial rewrite was fairly straight forward. Upon each request, the handler would parse the request details and if valid, spawn a blocking task to call into the C++ API and do the work. The blocking task used a fixed size, thread local buffer, and then the part of the buffer that had been written got copied into a new `Vec<u8>` and returned to the async task where the response was constructed.

This was using Actix-web, which means the blocking workers are assocaited with each async worker, but that doesn't make a very big difference.

## the performance test

Our performance tests operate in ramp up stages, where each test phase adds additional requests from the load generator and then measures the response time and tracks the total QPS as well as recording replica CPU and Memory. We're usually looking at the QPS we can achieve within a specific p98 latency.

Here is what the test results looked like for our original implementation.

![Graph showing 10 test phases. For each phase a stacked bar shows average and p98 latency and a line shows throughput in QPS. The Bars start small, under 100ms, while the throughput grows. From phase 5 the latency jumps up about 4x and then keeps growing in each stage until the p98 is around 500ms and the average is around 200ms. The throughput from phases 5 to 10 stays flat at around 550 QPS.](/img/max-520-qps/qps_during_ramps-python.png)

The response time (for the bar charts) is on the left axis and the throughput in queries per second (for the line chart) is on the right axis. The response times are all in milliseconds, even though the axis omits this.

We see pretty low response times (which is good) up until Test Phase 04, then they start growing quickly while the throughput remains basically flat for phases 5 to 10. This indicates that our system is unable to handle more than around 500 QPS.

So, let's look at the results for the rewrite.

![Graph showing 10 test phases. The values are very similar to  The graph For each phase a stacked bar shows average and p98 latency and a line shows throughput in QPS. The Bars start small, under 100ms, while the throughput grows. From phase 5 the latency jumps up about 4x and then keeps growing in each stage until the p98 is around 500ms and the average is around 200ms. The throughput from phases 5 to 10 stays flat at around 550 QPS.](/img/max-520-qps/qps_during_ramps-rust_1.png)

And it's basically the same graph... Which shouldn't be a surprise to you, because I said this at the beginning of the post. But it was surprising to me.

However, looking a little further, I did find one important difference, the CPU usage.

![Two line graphs showing CPU usage across time. The upper one is labled "Python" and reaches a bit under 120% around half way through the width of the graph. The lower one is labled "Rust" and has a similar form, but only reaches around 65%.](/img/max-520-qps/cpu_comparison_python_rust.png)

Both graphs follow a similar pattern with a constant increase from 0 during roughly half the test time, then they both level out. The Python graph shows some jitter which is due to adding another worker, but it's not so important because then it levels out. The big difference is that the Rust chart shows only a bit over **half** the maximum CPU usage of the Python chart (65% vs. 120%)!

Now, using half the CPU to do the same thing is often great. But I didn't want to use half the CPU, I wanted to use **all** the available CPUs and increase throughput.

Investigation Time!

## is it slow locally?

Our production application runs in Kubernetes, so the replicas I talked about above are pods. The performance tests also run on Kubernetes, so it's very close to the real thing.

But to test elsewhere, I tried running the applications locally. Well, locally on an EC2 instance. I used the same family of CPU as the Kubernetes node group we run in production, although a generation or 2 better.

So I tried running performance tests using [Locust] from the same host. This means there are all sorts of differences compared to our _real_ performance tests running on Kubernetes, but I was hoping it would give me a good idea. As far as I can tell, Locust doesn't have a similar "ramp up" to what we use, so I set it to 30 "users" sending requests from a pool of 3000 different requests and let it go.

[Locust]: https://locust.io/

The graph below shows the Locust results for the Rust implementation on the left and the Python implementation on the right.

![Locust web UI showing 3 graphs across time; Total requests per second, Response time, and Number of Users. There are 2 distinct sets of graphs across time. The first is labelled Rust and the second is labeled Python. The Rust graph shows slightly higher requests per second, slightly lower latency, and the same 30 users.](/img/max-520-qps/locust_test-python_rust.png)

Once again, the results are very similar. The Rust QPS is slightly higher, a bit above 800 while Python is a bit under and the Rust average and p95 latency is perhaps a little lower.

But what really stands out is that the average and p95 latency are both very low compared to the performance tests in Kubernetes. We have to consider that running from the same host there's far less network latency, but all the same, the average (p50) latency stays under 10ms for both implementations and the p95 is mostly under 20ms. This is much better than we saw in Kubernetes.

But how could I be sure that having access to 8 cores (even if only 2 workers were configured) wasn't making all the difference. I had my doubts about the validity of this test, but at least I had something here that showed that we weren't really capped at 520 QPS.

## is it the load balancer?

Our application runs behind a gateway that handles request authentication and authorization, that gateway runs behind an internet facing [AWS ALB].

[AWS ALB]: https://aws.amazon.com/elasticloadbalancing/application-load-balancer/

Our performance tests run in the same way to make them as close to production as possible. However, in this case the requests are coming from a load generator which is sitting in the same Kubernetes cluster. It looks like this:

![Flow diagram showing requests going from a Load Generator (inside Kubernetes Namespace) to an ALB then to the Auth Gateway and then finally to the Application Pod (also inside Kubernetes Namespace).](/img/max-520-qps/app_architecture.svg)

I have to trust the ALB, not just because it's from AWS, but because we run much higher loads through them without issue.

The same goes for the authentication gateway really. In the performance tests we're running against only a single replica, but in production we run against many more and generally don't have any problems. Only 520 QPS should be handled easily.

I did do a bit of digging. The authentication gateway uses [nginx] underneath. The most likely culprit would be that `worker_connections` is set too low. But I found values around 10K everywhere, so that wasn't it either.

[nginx]: https://nginx.org/

In the end, I gave up searching through nginx configuration values and decided to just take it all out of the equation. Having the load generator and the application pod in the same namespace meant that I could cut both the ALB and the authentication gateway out of the picture.

![Flow diagram showing requests going from a Load Generator (inside Kubernetes Namespace) to an ALB then to the Auth Gateway and then finally to the Application Pod (also inside Kubernetes Namespace).](/img/max-520-qps/simplified_architecture.svg)

## is the hand-break off?

Discussing this issue with some friends, I flippantly stated that I assumed that [Nagle's algorithm] had been turned off. I've not personally had a problem that turned out to be due to Nagle, but apparently [It’s always `TCP_NODELAY`]. I also know that both the Rust `stdlib` and Tokio don't turn off Nagle (or turn on `TCP_NODELAY`, which is what the action is) by default. I had also heard that the [Tokio] team had often had to recommend this to people who were complaining that their async Rust implementation was slower than whatever else they were comparing it to.

[Nagle's algorithm]: https://en.wikipedia.org/wiki/Nagle%27s_algorithm
[It’s always `TCP_NODELAY`]: https://brooker.co.za/blog/2024/05/09/nagle.html
[Tokio]: https://tokio.rs

Well, it turns out that Actix-web doesn't turn `TCP_NODELAY` on by default either, but it's easy enough to do so ([`HttpServer::tcp_nodelay`]). So I did that, and then also turned on `TCP_QUICKACK` for good measure (Nagle's algorithm plays notoriously badly with delayed acks and for our use case I don't see any good reason to leave it off).

[`HttpServer::tcp_nodelay`]: https://docs.rs/actix-web/latest/actix_web/struct.HttpServer.html#method.tcp_nodelay

First I tested the change with Locust (because that was faster), this time we're only testing the Rust implementation and the difference is having `TCP_NODELAY` and `TCP_QUICKACK` disabled or enabled.

Here you can see that I'd upped the number of "users" to 80 and also modified the response time chart to show p98 and p99 instead of p95. You're getting these inconsistent charts because this all happened a couple of months ago and I'm salvaging the few screenshots I still have lying around for this post.

![Locust web UI showing 3 graphs across time; Total requests per second, Response time, and Number of Users. There are 2 distinct sets of graphs across time. The first shows `TCP_NODELAY` and `TCP_QUICKACK` disabled and the second shows with both enabled. With the options enabled we see significantly higher QPS (from 1700 disabled to mostly over 2000 disabled), but the QPS is also much more variant enabled. The response times, especially the p98 and p99 times which go from 60ms disbaled to 20-40ms enabled. The number of users is 80 in both cases.](/img/max-520-qps/locust_test-rust_nagle.png)

Finally, we see a difference. The QPS and response time do seem to vary a lot more with the 2 options enabled (and I don't know why), but the QPS is significantly higher and the p98 and p99 latency down by half. This is great, will I get the same results in our real performance tests?

No.

There was basically no change. I'm not even going to show a graph, because it's the same.
