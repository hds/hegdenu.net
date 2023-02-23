use tracing::Subscriber;
use tracing::{error, info};
use tracing_subscriber::{prelude::*, Layer};

struct HtmlFormatterLayer;

impl<S> Layer<S> for HtmlFormatterLayer where S: Subscriber {}

#[derive(Debug)]
struct Mog {
    val: u32,
}

fn main() {
    tracing_subscriber::fmt()
        .with_line_number(false)
        .with_file(true)
        .init();

    let mog = Mog { val: 42 };
    error!(?mog, "Some error with mog.");

    info!("Hello, world!");
}
