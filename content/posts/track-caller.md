+++
title = "track caller"
author = "hds"
date = "2023-01-29"
+++

I've recently contributed a little bit to the [Tokio](https://github.com/tokio-rs/tokio) project.

The first issue I worked on was [#4413](https://github.com/tokio-rs/tokio/issues/4413): *polish: add `#[track_caller]` to functions that can panic*.

Now I'm going to tell you everything you didn't know that you didn't need to know about `#[track_caller]`.

### what

Before Rust 1.42.0 errors messages from calling `unwrap()` weren't very useful.

You would get something like this:

```
thread 'main' panicked at 'called `Option::unwrap()` on a `None` value', /.../src/libcore/macros/mod.rs:15:40
```

This tells you nothing about *where* `unwrap` panicked.

This was improved for `Option::unwrap()` and `Result::unwrap()`in Rust [1.42.0](https://blog.rust-lang.org/2020/03/12/Rust-1.42.html#useful-line-numbers-in-option-and-result-panic-messages).

More interestingly, the mechanism to do this was stablised in Rust [1.46.0](https://blog.rust-lang.org/2020/08/27/Rust-1.46.0.html#track_caller).

What was this mechanism? The [`track_caller`](https://doc.rust-lang.org/reference/attributes/codegen.html#the-track_caller-attribute) attribute.

### how

Where would you use `#[track_caller]`?

Imagine you're writing a library, it's called `track_caller_demo`.

Here's the whole thing:

```rust
/// This function will return non-zero values passed to it.
/// 
/// ### Panics
/// 
/// This function will panic if the value passed is zero.
pub fn do_not_call_with_zero(val: u64) -> u64 {
    if val == 0 {
        panic!("We told you not to do that");
    }

    val
}
```

We have been quite clear - you [MUST NOT](https://www.ietf.org/rfc/rfc2119.txt) pass zero to this function.

Now along comes a user of your crate and writes this code:

```rust
use track_caller_demo::do_not_call_with_zero;

fn code_written_by_crate_user() {
    do_not_call_with_zero(0);
}
```

When the user runs their code, they'll see the following:

```
thread 'main' panicked at 'We told you not to do that', .cargo/registry/src/github.com-1ecc6299db9ec823/track_caller_demo-0.1.0/src/lib.rs:8:9
```

And the user says, "the crate author wrote buggy code!"

But we told them not to pass zero to that function.

We did it in multiples ways.

We don't want the user to see where the code panicked in **our** crate.

We want to show them **their** mistake.

So we annotate our function with `#[track_caller]`:

```rust
/// This function will return non-zero values passed to it.
/// 
/// ### Panics
/// 
/// This function will panic if the value passed is zero.
#[track_caller]
pub fn do_not_call_with_zero(val: u64) -> u64 {
    if val == 0 {
        panic!("We told you not to do that");
    }

    val
}
```

Now the user will see the following error message instead:

```
thread 'main' panicked at 'We told you not to do that', src/bin/zero.rs:4:5
```

This shows the location in the user's code where they called our library incorrectly.

Success!

### except

There is one caveat.

The `track_caller` attribute must be on the whole call stack.

Every function from the panic, upwards.

Otherwise it won't work.

Let's add a new function to our library:

```rust
/// This function will return non-one values passed to it.
/// 
/// ### Panics
/// 
/// This function will panic if the value passed is one.
#[track_caller]
pub fn do_not_call_with_one(val: u64) -> u64 {
    panic_on_bad_value(val, 1);

    val
}

fn panic_on_bad_value(val: u64, bad: u64) {
    if val == bad {
        panic!("We told you not to provide bad value: {}", bad);
    }
}
```

We annotate our public function with `#[track_caller]`.

Let's check the output:

```
thread 'main' panicked at 'We told you not to do that', .cargo/registry/src/github.com-1ecc6299db9ec823/track_caller_demo-0.1.0/src/lib.rs:29:9
```

The panic is pointing at our perfectly good library code!

To make this work, annotate the whole stack:

```rust
/// This function will return non-one values passed to it.
/// 
/// ### Panics
/// 
/// This function will panic if the value passed is one.
#[track_caller]
pub fn do_not_call_with_one(val: u64) -> u64 {
    panic_on_bad_value(val, 1);

    val
}

#[track_caller]
fn panic_on_bad_value(val: u64, bad: u64) {
    if val == bad {
        panic!("We told you not to provide bad value: {}", bad);
    }
}
```

Now we get:

```
thread 'main' panicked at 'We told you not to provide bad value: 1', src/bin/one.rs:4:5
```

Much better!

Most of the work on [tokio#4413](https://github.com/tokio-rs/tokio/issues/4413) was writing tests to ensure this didn't happen.

### except except

OK, there's **another** caveat.

The `track_caller` attribute doesn't work in some places.

It doesn't work on closures ([rust#87417](https://github.com/rust-lang/rust/issues/87417)).

But it does newly work on async functions ([rust#78840](https://github.com/rust-lang/rust/issues/78840)).

Although I can't seem to work out which version of Rust that's in.