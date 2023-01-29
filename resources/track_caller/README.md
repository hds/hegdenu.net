# track_caller

A small demo to go with the track caller post.

## Running

There's nothing much to it. Run the example in the section **how** with:

```sh
cargo run --bin zero
```

The example will panic:

```
thread 'main' panicked at 'We told you not to do that', src/main.rs:4:5
```

For the example in the section **except**, run:

```sh
cargo run --bin one
```