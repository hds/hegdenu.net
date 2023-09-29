# tracing-tokio-tasks

This is the sample code for the
[tracing tokio tasks](https://hegdenu.net/posts/tracing-tokio-tasks/) blog
post.

## Run

You can run it with cargo.

```sh
cargo run --package tracing-tokio-tasks
```

### Pretty output

If you don't want the HTML output, you need to comment out the following line
in [main.rs](src/main.rs):

```rust
        .map_writer(|w| move || HtmlWriter::new(w()))
```

You will get a warning about an unused import, but that can be ignored.