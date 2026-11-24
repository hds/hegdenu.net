+++
title = "max 560 qps"
slug = "max-560-qps"
author = "hds"
date = "2026-11-24"
+++

I had a fun riddle a few months ago, a webserver rewrite from Python to Rust that used half the CPU, but showed the same throughput in performance tests.

## the setup

We've got a Python webserver ([Tornado]) that wraps a C++ library that does pretty much all the work.

[Tornado]: https://www.tornadoweb.org

Python passes in validated request parameters and a pointer to a byte buffer, C++ looks up content in an in-memory index and then writes a protobuf response directly into the buffer. Python then returns the buffer contents as the response body.

## the original problem

This rewrite wasn't for the sake of a rewrite. We had some real performance issues in production.

We auto-scale this particular service based on CPU usage. We had to set the target fairly low, because we hadn't been able to achieve very good parallelism, but we had learned to live with this. However, after an increase in the volume of data in our in-memory index and therefore also in responses, we found that when we lost a replica (for example, lost 1 out of 6 replicas), we were seeing big latency spikes and even some requests timing out.

With 2 CPUs to serve requests, we had already set our auto-scaling target at 120% (out of a maximum of 200% because we have 2 CPUs). To mitigate these new spikes, we dropped it down to 100%. The mitigation worked, but this was something that needed to be fixed!

Each of these replicas was still serving 500-600 QPS, so not a push over by any means, but we should be able to get more out of 2 CPUs.

## the prototype

This is the only bit of Python we have in our production stack, and I wasn't really sure how best to go about investigating why we couldn't get more CPU utilization.

Being Python, it could be a GIL (Global Interpreter Lock) problem, but I didn't want to go blaming the GIL without some sort of proof. I looked into tools to profile the GIL, but I didn't find anything that looked like it would help me, not a Python performance expert, determine whether the GIL was really to blame or not.

We're mostly a Rust team now, so rewriting the Python part in Rust would give us a simplified stack and allow us to use some of our existing tooling for this service.

So I prototyped a rewrite, converting the Python layer (which isn't very large anyway) to Rust. I ended up playing around a bunch with possible optimizations, but the initial rewrite was fairly straightforward. Upon each request, the handler would parse the request details and if valid, spawn a blocking task to call into the C++ API and do the work. The blocking task used a fixed size, thread local buffer, and then the part of the buffer that had been written got copied into a new `Vec<u8>` and returned to the async task where the response was constructed.

This was using Actix-web, which means the blocking workers are assocaited with each async worker, but that doesn't make a very big difference.

## the performance test

Our performance tests operate in ramp up stages, where each test phase adds additional requests from the load generator and then measures the response time and tracks the total QPS as well as recording replica CPU and Memory. We're usually looking at the QPS we can achieve within a specific p98 latency.

Here is what the test results looked like for our original implementation.

![Graph showing 10 test phases. For each phase a stacked bar shows average and p98 latency and a line shows throughput in QPS. The Bars start small, under 100ms, while the throughput grows. From phase 5 the latency jumps up about 4x and then keeps growing in each stage until the p98 is around 500ms and the average is around 200ms. The throughput from phases 5 to 10 stays flat at around 560 QPS.](/img/max-560-qps/qps_during_ramps-python.png)

The response time (for the bar charts) is on the left axis and the throughput in queries per second (for the line chart) is on the right axis. The response times are all in milliseconds, even though the axis omits this.

We see pretty low response times (which is good) up until Test Phase 04, then they start growing quickly while the throughput remains basically flat for phases 5 to 10. This indicates that our system is unable to handle more than some value around 560 QPS.

So, let's look at the results for the rewrite.

![Graph showing 10 test phases. The values are very similar to  The graph For each phase a stacked bar shows average and p98 latency and a line shows throughput in QPS. The Bars start small, under 100ms, while the throughput grows. From phase 5 the latency jumps up about 4x and then keeps growing in each stage until the p98 is around 500ms and the average is around 200ms. The throughput from phases 5 to 10 stays flat at around 560 QPS.](/img/max-560-qps/qps_during_ramps-rust_1.png)

And it's basically the same graph... Which shouldn't be a surprise to you, because I said this at the beginning of the post. But it was surprising to me at the time!

However, looking a little further, I did find one important difference, the CPU usage.

![Two line graphs showing CPU usage across time. The upper one is labled "Python" and reaches a bit under 120% around half way through the width of the graph. The lower one is labled "Rust" and has a similar form, but only reaches around 65%.](/img/max-560-qps/cpu_comparison_python_rust.png)

Both graphs follow a similar pattern with a constant increase from 0 during roughly half the test time, then they both level out. The Python graph shows some jitter which is due to adding another worker, but it's not so important because then it levels out. The big difference is that the Rust chart shows only a bit over **half** the maximum CPU usage of the Python chart (65% vs. 120%)!

Now, using half the CPU to do the same thing is often great. But I didn't want to use half the CPU, I wanted to use **all** the available CPUs and increase throughput.

Investigation Time!

## is it slow locally?

Our production application runs in Kubernetes, so the replicas I talked about above are pods. The performance tests also run on Kubernetes.

To test in a faster loop, I tried running the applications locally. Well, "locally" on an EC2 instance. I used the same family of CPU as the Kubernetes node group we run in production, although a generation or 2 better.

So I tried running performance tests using [Locust] from the same host. This means there are all sorts of differences compared to our _real_ performance tests running on Kubernetes, but I was hoping it would give me a good idea. As far as I can tell, Locust doesn't have a similar "ramp up" to what we use, so I set it to 30 "users" sending requests from a pool of 3000 different requests and let it go.

[Locust]: https://locust.io/

The graph below shows the Locust results for the Rust implementation on the left and the Python implementation on the right.

![Locust web UI showing 3 graphs across time; Total requests per second, Response time, and Number of Users. There are 2 distinct sets of graphs across time. The first is labelled Rust and the second is labeled Python. The Rust graph shows slightly higher requests per second, slightly lower latency, and the same 30 users.](/img/max-560-qps/locust_test-python_rust.png)

Once again, the results are very similar. The Rust QPS is slightly higher, a bit above 800 while Python is a bit under and the Rust p50 and p95 latency is perhaps a little lower. Note that in Locust I'm using the p50, but our performance testing framework uses average insead.

But what really stands out is that the p50 and p95 latency are both very low compared to the performance tests in Kubernetes. We have to consider that running from the same host there's far less network latency, but all the same, the p50 latency stays under 10ms for both implementations and the p95 is mostly under 20ms. This is much better than we saw in Kubernetes.

But how could I be sure that having access to 8 cores (even if only 2 workers were configured) wasn't making all the difference. I had my doubts about the validity of this test, but at least I had something here that showed that we weren't really capped at 560 QPS.

## is it the load balancer?

Our application runs behind a gateway that handles request authentication and authorization, that gateway runs behind an internet facing [AWS ALB].

[AWS ALB]: https://aws.amazon.com/elasticloadbalancing/application-load-balancer/

Our performance tests run in the same way to make them as close to production as possible. However, in this case the requests are coming from a load generator which is sitting in the same Kubernetes cluster. It looks like this:

![Flow diagram showing requests going from a Load Generator (inside Kubernetes Namespace) to an ALB then to the Auth Gateway and then finally to the Application Pod (also inside Kubernetes Namespace).](/img/max-560-qps/app_architecture.svg)

I have to trust the ALB, not just because it's from AWS, but because we run much higher loads through them without issue.

The same goes for the authentication gateway really. In the performance tests we're running against only a single replica, but in production we run against many more and generally don't have any problems. Only 560 QPS should be handled easily.

I did do a bit of digging. The authentication gateway uses [nginx] underneath. The most likely culprit would be that `worker_connections` is set too low. But I found values around 10K everywhere, so that wasn't it either.

[nginx]: https://nginx.org/

In the end, I gave up searching through nginx configuration values and decided to just take it all out of the equation. Having the load generator and the application pod in the same namespace meant that I could cut both the ALB and the authentication gateway out of the picture.

![Flow diagram showing requests going from a Load Generator (inside Kubernetes Namespace) to an ALB then to the Auth Gateway and then finally to the Application Pod (also inside Kubernetes Namespace).](/img/max-560-qps/simplified_architecture.svg)

Unfortunately, this made no difference what-so-ever. I'm not even going to show a graph, because it's the same.

## is the handbreak off?

Discussing this issue with some friends, I flippantly stated that I assumed that [Nagle's algorithm] had been turned off. I've not personally had a problem that turned out to be due to Nagle, but apparently [It’s always `TCP_NODELAY`]. I also know that both the Rust `stdlib` and Tokio don't turn off Nagle (or turn on `TCP_NODELAY`, which is what the action is) by default. I had also heard that the [Tokio] team had often had to recommend this to people who were complaining that their async Rust implementation was slower than whatever else they were comparing it to.

[Nagle's algorithm]: https://en.wikipedia.org/wiki/Nagle%27s_algorithm
[It’s always `TCP_NODELAY`]: https://brooker.co.za/blog/2024/05/09/nagle.html
[Tokio]: https://tokio.rs

Well, it turns out that Actix-web doesn't turn `TCP_NODELAY` on by default either, but it's easy enough to do so ([`HttpServer::tcp_nodelay`]). So I did that, and then also turned on `TCP_QUICKACK` for good measure (Nagle's algorithm plays notoriously badly with delayed acks and for our use case I don't see any good reason to leave it off).

[`HttpServer::tcp_nodelay`]: https://docs.rs/actix-web/latest/actix_web/struct.HttpServer.html#method.tcp_nodelay

First I tested the change with Locust (because that was faster), this time we're only testing the Rust implementation and the difference is having `TCP_NODELAY` and `TCP_QUICKACK` disabled or enabled.

Here you can see that I'd upped the number of "users" to 80 and also modified the response time chart to show p98 and p99 instead of p95. You're getting these inconsistent charts because this all happened a couple of months ago and I'm salvaging the few screenshots I still have lying around for this post.

![Locust web UI showing 3 graphs across time; Total requests per second, Response time, and Number of Users. There are 2 distinct sets of graphs across time. The first shows `TCP_NODELAY` and `TCP_QUICKACK` disabled and the second shows with both enabled. With the options enabled we see significantly higher QPS (from 1700 disabled to mostly over 2000 disabled), but the QPS is also much more variant enabled. The response times, especially the p98 and p99 times which go from 60ms disbaled to 20-40ms enabled. The number of users is 80 in both cases.](/img/max-560-qps/locust_test-rust_nagle.png)

Finally, we see a difference. The QPS and response time do seem to vary a lot more with the 2 options enabled (and I don't know why), but the QPS is significantly higher and the p98 and p99 latency down by half. This is great, will I get the same results in our real performance tests?

No.

There was basically no change. Once again, I'm not going to show the graph, it's the same.

I tried a few more typical approaches to improving performance. I tried configuring the pods to meet the requirements for Kubernetes's [Guaranteed Quality of Service class](https://kubernetes.io/docs/concepts/workloads/pods/pod-qos/#guaranteed), doubling the available cores from 2 to 4, and then finally just adding another replica. When that last one failed, it looked more like the web server wasn't the problem, a theory I needed to test further.

## what if the server was perfectly fast?

I was starting to run out of reasonable ideas, so I turned to the unreasonable ones.

What if I made the sever perfectly fast? I modified my Rust port to just return an HTTP 204 from the handler without doing anything at all. It's not really important, but this is a _perfectly_ valid response from the service that I had rewritten.

With that change, we see a graph that is just a little bit different, but that difference turns out to be important.

![Graph showing 10 test phases. For each phase a stacked bar shows average and p98 latency and a line shows throughput in QPS. The Bars start so small they almost aren't there at all, under 5ms, while the throughput grows. From phase 5 the latency jumps up to around 200ms p98 then keeps growing in each stage until the p98 is around 600ms and the average is around 100ms. The throughput from phases 5 to 10 grows slowly from around 550 to 700 QPS.](/img/max-560-qps/perfect_fast_server-qps.png)

The pattern is similar, we have low latency for the first 4 phases and then jumps up and grows from there. The difference is that in the first 4 phases the latency is very low, in Test Phase 04 we see a p98 of 3ms. This is as expected, the web server is doing very little. 

But we still see the jump, now a huge jump, in latency from Test Phase 05 onwards. From Test Phase 04 to Test Phase 05 the p98 latency went from 3ms to close to 200ms.

Looking at the CPU, we can see that the webserver was indeed doing very little.

![Line graph showing CPU usage across time. The value starts close to zero and increases up to around 12% around 3/4 of the way through the timeline before flattening off.](/img/max-560-qps/perfect_fast_server-cpu.png)

The CPU usage only reaches around 12%! At this point, I had to put aside my normal reservations about blaming some other component that doesn't belong to us - I needed to take a closer look at the performance testing framework.

## is the load generator getting throttled?

I started looking at the performance tests, specifically at the load generator.

Our performance tests are standardized so that we only need to configure the request pool and the resources for the load geneators. As I described earlier, the load generators are run from the same Kubernetes namespace as the application, but they run on a dedicated node group so they're always on different instances. The load generator software is [Apache JMeter](https://jmeter.apache.org/).

Despite running on a dedicated node group, I suspected that we may be getting throttled. We had CPU [requests and limits](https://kubernetes.io/docs/concepts/configuration/manage-resources-containers/#requests-and-limits) set to 1 and 1.5 respectively for the load generator. So I checked the kubernetes metrics which we conveniently have in Grafana.

![Grafana panel showing CPU usage and Throttled CPU across roughly 40 minutes. After around 8 minutes of low CPU usage and no throttling the CPU usage increases roughly linearly to 1.25 and the throttled CPU increases to 0.7These load generators ](/img/max-560-qps/load_generator-throttled.png)

We do see throttling, and much more than I expected. We start seeing CPU throttling when the CPU usage is only at around 0.3 (30%) and as the CPU usage grows across time to 1.25, the throttling grows to 0.7, which is far more than I expected.

When a container has its CPU throttled in Kubernetes, it is performed in 100ms increments. So if a container has exceeded its limit, it won't be scheduled for 100ms. This is a huge amount of time when we're looking at an overall p98 latency under 40ms (and worse if you look at the "perfectly fast" server with a p98 of 3ms). An interesting details is that this isn't a Kubernetes implementation detail, it's the default value for the Completely Fair Scheduler (CFS) in the Linux kernel, controled by the interface file [`cpu.cfs_period_us`](https://docs.kernel.org/scheduler/sched-bwc.html#management) (for cfgroup v1, cfgroup v2 has a different configuration format, but the default is the same).

I went about increasing the requests and limits in an attempt to avoid getting throttled. I also configured the load generator pods so that they would meet the requirements for the [Guaranteed Quality of Service class](https://kubernetes.io/docs/concepts/workloads/pods/pod-qos/#guaranteed) (as you can see, this is a common theme). The load driver container got 4 CPUs and 12GiB memory, which should have been plenty.

This removed the throttling, but it didn't really have the expected effect. The QPS got a bit higher, and the latency was lower, but the characteristic jump from Test Phase 04 (which was still at around 3ms) to Test Phase 05 was still present. So, throttling partially explained the latency, but not the cap on the throughput (QPS).

## is the load generator just not able to generate that much load?

Somehow, the load generator didn't seem able to generate enough load to test our web server.

Locust, the load testing tool I'd used locally, will [print a warning](https://docs.locust.io/en/stable/increasing-request-rate.html#load-generation-performance) about high CPU usage, indicating that it requires additional resources to correctly generate and measure load. Unfortunately, it seems like JMeter doesn't do this, or at least it's not exposed in our load testing framework. It would have saved me a lot of time. Instead, what I saw was the slow down in JMeter being attributed to the system under test.

_insert meme: you had one job..._

Fortunately, I could scale horizontally. This is cloud native after all, it's all about horizontal scaling of things that scale horizontally. And a stateless load generator scales horizontally very well.

I reduced the load generated by each individual load generator to not reach the QPS we'd observed in Test Phase 05 - specifically, so that each load generator wouldn't generate more than 500 QPS (less than the mysterious 560 QPS limit we'd found at the beginning). Then I ran 3 load generators together and reduced their CPU count back down to 2 each as they weren't getting throttled any more. I also switched back to the web server that actually served data, not the cheating one.

This finally gave me the graph I was looking for.

![Graph showing 10 test phases. For each phase a stacked bar shows average and p98 latency and a line shows throughput in QPS. The line for QPS grows lineaerly from Test Phase 02 to Test Phase 10, reaching over 1200. The p98 latency starts off linear and then jumps up a little bit in Test Phase 09 and Test Phase 10. The average latency is linear, and remains relatively low throughout.](/img/max-560-qps/load_generators_x3-qps.png)

We now see the QPS scaling pretty much linearly with the test phases and it reaches a bit over 1200 QPS by Test Phase 10. The latency starts off low (around 10ms p98) and then from Test Phase 05 to Test Phase 08 it grows up to 100ms, after that it climbs faster, reaching 250ms p98 latency by Test Phase 10. The average latency grows linearly, staying below 30ms even in Test Phase 10.

The graph of the web server CPU usage is also close to what I would expect.


![A line graph showing CPU usage, it starts at zero and grows roughly linearly until it reaches 160](/img/max-560-qps/load_generators_x3-cpu.png)

I'm now finally able to actually test the web server, not just the limits of the load generator. Of course, we now have 3 2-core load generators, to test a single 2-core web server. My feeling is that we're not configuring JMeter correctly somehow, I don't believe this is the best throughput we can get out of it. However, at this point being able to properly load test our own code is the most important part. We could probably gain some cost saving if we can improve the JMeter configuration to not need so many generators, but they don't run for very long, so it's not a top priority.

## what about that rust rewrite?

Now that I had functioning performance tests I could see that my Rust rewrite wasn't quite the super efficient solution that I was aiming for. I already had a few ideas for how to improve the Rust port. To being with, spawning a blocking task for every request likely wasn't ideal.

However, in the meantime, we'd solved some of the problems with our Python Tornado server getting overloaded. Specifically, the default value for `backlog` (the size of the queue for unattended connections), which is documented as a [reasonable value](https://docs.python.org/3/library/socket.html#socket.socket.listen) is too low for our purposes. By signficantly increasing the value, we no longer got overloaded. A few more improvements in the Python to C++ interface allowed us to reduce our production p98 latency by half, so getting significant improvements by switching to Rust is now harder than it had been. And thanks to the results of this investigation, we were actually able to see those improvements in our performance tests!

We still have reasons to switch to a Rust implementation in the future, but that is a story for another day.
