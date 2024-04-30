+++
title = "kubernetes memory"
slug = "kubernetes-memory"
author = "hds"
date = "2024-05-31"
draft = true
+++

I've been trying to get my head around the memory metrics reported by kubernetes and what the numbers mean. Especially since the gaps caused us problems yesterday. (Thanks to Maksym and Chris for explaining!)

This is the best analysis I've found (thanks Ellie!):
- https://itnext.io/from-rss-to-wss-navigating-the-depths-of-kubernetes-memory-metrics-4d7d77d8fdcb

But these two also helped:
- https://blog.freshtracks.io/a-deep-dive-into-kubernetes-metrics-part-3-container-resource-metrics-361c5ee46e66
- https://mohamedmsaeed.medium.com/memory-working-set-vs-memory-rss-in-kubernetes-which-one-you-should-monitor-8ef77bf0acee

This is my visual representation of memory statistics:

![Concentric circles showing memory metrics. Inner circle is labelled "rss". Next larger circle labelled "active pages" indicating that it is only the larger part of the circle excluding the smaller circle. Largest circle labelled "inactive pages", again indicating that only the difference between the largest and inner circle is included. Three additional circles are outside indicating total amounts. "wss" covers all the space of "active pages" and "rss". "usage" covers all the space of the three main circles: "inactive pages", "active pages", and "rss". "memory cache" covers "inactive pages" and "active pages", excluding "rss".](/img/kubernetes-memory/memory.png)


Where the following metrics are available:

* `container_memory_working_set_bytes` - working set size (wss)
* `container_memory_rss` - resident set size (rss)
* `container_memory_usage_bytes` - usage
* `container_memory_cache` - memory cache

There's no way to get active and inactive page sizes per pod / container that I'm aware of.

