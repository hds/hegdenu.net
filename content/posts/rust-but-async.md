+++
title = "rust but async"
author = "hds"
date = "2024-02-27"
draft = false
+++

One of the things that is said about Rust is that it's the result of paying attention to the last 30 years of research into programming language design. This is probably being unfair to some other languages - even if only by omission. However, we can say that Rust has definitely made the right choices in some places where no widely successful programming language has done before.

One of the other reasons for Rust's success up until now is likely the broad range of areas in which it can be used. Yes, you can use it for systems programming, but Rust is also a great choice for writing a command line tool, a backend web application, and of course game development is in there as well. It may well be this broad applicability that has given Rust the critical mass necessary to be successful without a large tech company basically forcing it upon developers. Swift is the obvious case here, but Kotlin fits the mold and even Go when it comes to using Kubernetes.

That is an interesting lens to look at [Mojo](https://www.modular.com/max/mojo) through.

### rust, but for AI

Mojo is a new programming language from [Modular](https://www.modular.com) a company co-founded by [Chris Lattner](https://en.wikipedia.org/wiki/Chris_Lattner). Lattner created [LLVM](https://llvm.org/) (the compiler toolchain that Rust uses as a backend) as part of his master research and then later worked at Apple where he created [Swift](https://www.swift.org/).

For me, the interesting thing about Mojo is the way it is being positioned. On the one hand, the [landing page](https://www.modular.com/max/mojo) calls it "the programming language _for all AI developers_" (emphasis is theirs). On the other hand, a [recent blog post](https://www.modular.com/blog/mojo-vs-rust-is-mojo-faster-than-rust) from the Modular compares Mojo to Rust, mostly in terms of developer ergonomics and execution performance. What I took away from that blog post is that Modular is positioning Mojo as **Rust, but for AI**.

This is based on Modular running [an AI platform](https://www.modular.com/) and the linked blog post putting a lot of emphasis on both AI use cases and the reticence of data scientists to learn a language that is different from their primary tool today, which is Python. From this point of view, the Rust, but for AI argument makes sense (when talking to a certain audience). Today, much of AI/ML and data science in general run on Python for the front end and C/C++ for the backend because Python is too slow. There aren't a lot of languages in a position to insert themselves there, but Rust could become one - after a bunch of necessary work, especially on the GPU side.

This specificity seems to go against one of the things that I believe has made Rust successful. But Mojo will have the corporate push (and may have the right niche) to build Mojo up despite this.

### rust, but for X

This leads to the whole point of this post. If you could have Rust, but for _something in particular_, then you could probably cut corners to improve the language for that use case - the flip side is that you may make it unusable for other uses cases.

One of the things I use Rust for is backend services (that serve _stuff_,  what stuff isn't really important). In 2024 that (mostly) means concurrent programming, which means async Rust. So what if we took Rust and made another programming language, one that sacrificed other use cases and made it the best possible Async Rust. What would that look like?

### asr

For lack of a better name, let's call this language ASR (something like ASyncRust - whatever, you can call it a better name in your head if you like).

ASR is the same as Rust, but with a few small (or kind of small) changes.

#### everything async

Let's go back to that famous blog post [What Color is Your Function?](https://journal.stuffwithstuff.com/2015/02/01/what-color-is-your-function/). It posits that async-await in Javascript helps (but doesn't completely solve) the problem with async functions there (and in many other languages), which is that you have to treat async functions (red) differently from normal functions (blue). And the worst thing is, while you can call a normal (blue) function from an async (red) function, you can't do it the other way around. Which is true in async Rust and often a cause of problems.

So let's just do away with "normal" functions. In [ASR](#asr), we'll make everything async instead. There is no async-await syntax either, because all functions are async and every time you call a function there is some implied awaiting happening.

Of course, all these async functions are actually still futures underneath - if you want to know more about how **that** works, start with [how I finally understood async/await in Rust (part 1)](@/posts/understanding-async-await-1.md).

We're going to depend on a smart compiler to optimise some of this away for us, in the same way that we depend on the compiler to optimise away some function calls by inlining.

#### async clean-up

Boats has been discussing a number of API concerns regarding async recently (well, for longer than that, but their posts have been coming thick and fast this month). The latest of those at the time of writing discusses [Asynchronous clean-up](https://without.boats/blog/asynchronous-clean-up/).

The article goes into great depth in a proposed solution, but a lot of the problems stem from async functions being different to normal functions. For example, a type with an async drop function can only be dropped in an async context - something which Rust doesn't currently have a way to check for (although the type system could likely support it in the future). This particular problem goes away in [ASR](#asr), where everything is async - since nothing happens outside an async context. There are nuances of course, but making everything async simplifies at least some problems.

#### structure

Rust's async-await syntax hides a lot of the complication of manually writing futures. This is especially true when it comes to holding references across await points (which turns into storing references inside a future). Just try implementing that manually and you'll see what I mean. However, borrowing becomes impossible once you start spawning tasks.

Tokio's [`spawn`](https://docs.rs/tokio/1.36.0/tokio/task/fn.spawn.html) requires that the future being spawned is `'static` (and [`smol`](https://docs.rs/smol/2.0.0/smol/fn.spawn.html) and [`async-std`](https://docs.rs/async-std/1.12.0/async_std/task/fn.spawn.html) have the same requirement). This means that it can't borrow anything from the surrounding context. The only way to pass references in is to `Arc` it up.

For OS threads, the Rust standard library solves this problem with [scoped threads](https://doc.rust-lang.org/std/thread/fn.scope.html), but the fact that futures can be cancelled means that the same solution doesn't extend to concurrent programming in Rust.

One solution to this problem would be structured concurrency. This is not a new idea and is already the standard in [Kotlin](https://kotlinlang.org/docs/coroutines-basics.html) and [Swift](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/concurrency/). With Rust's borrow checker, one would think that structured concurrency would be a natural fit, but it's not something that has yet found its way to a mainstream Rust async crate.

With structured concurrency in [ASR](#asr), we will ensure that parent tasks outlive their child tasks. For one, this would mean that if a parent task is cancelled, all its child tasks get cancelled before the parent task is truly cancelled (and dropped). In turn, this would allow us to propagate lifetimes to child tasks, removing the restriction that only `'static` futures can be spawned.

This is the one idea from this rant that I think is probably most interesting to explore in Rust today.

### what rust do you want?

That's more than enough pseudo language design from me for now.

As much as I am interested to hear all the ways in which the ideas I've presented in this post are impossible, I would be **much** more interested to hear what your own derivative of Rust would look like. Remember, dreaming is free!


