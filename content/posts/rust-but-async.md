+++
title = "rust but async"
author = "hds"
date = "2024-02-29"
draft = true
+++

One of the things that is said about Rust is that it's the result of paying attention to the last 30 years of research into programming language design. This is probably being unfair to some other languages - even if only by omission. However, we can say that Rust has definitely made the right choices in some places where no widely successful programming language has done before.

One of the other reasons for Rust's success up until now is likely the broad range of areas in which it can be used. Yes, you can use it for systems programming, but Rust is also a great choice for writing a command line tool, a backend web application, and of course game development is in there as well. It may well be this broad applicability that has given Rust the critical mass necessary to be successful without a large tech company basically forcing it upon developers. Swift is the obvious case here, but Kotlin fits the mold and even Go when it comes to using Kubernetes.

That is an interesting lens to look at [Mojo](https://www.modular.com/max/mojo) through.

## rust, but for AI

Mojo is a new programming language from [Modular](https://www.modular.com) a company co-founded by [Chris Lattner](https://en.wikipedia.org/wiki/Chris_Lattner). Lattner created [LLVM](https://llvm.org/) (the compiler toolchain that Rust uses as a backend) as part of his master research and then later worked at Apple where he created [Swift](https://www.swift.org/).

For me, the interesting thing about Mojo is the way it is being positioned. On the one hand, the [landing page](https://www.modular.com/max/mojo) calls it "the programming language _for all AI developers_" (emphasis is theirs). On the other hand, a [recent blog post](https://www.modular.com/blog/mojo-vs-rust-is-mojo-faster-than-rust) from the Modular compares Mojo to Rust, mostly in terms of developer ergonomics and execution performance. What I took away from that blog post is that Modular is positioning Mojo as **Rust, but for AI**.

This is based on Modular running [an AI platform](https://www.modular.com/) and the linked blog post putting a lot of emphasis on both AI use cases and the reticence of data scientists to learn a language that is different from their primary tool today, which is Python. From this point of view, the Rust, but for AI argument makes sense (when talking to a certain audience). Today, much of AI/ML and data science in general run on Python for the front end and C/C++ for the backend because Python is too slow. There aren't a lot of languages in a position to insert themselves there, but Rust could become one - after a bunch of necessary work, especially on the GPU side.

## rust, but for X

This leads to ...

