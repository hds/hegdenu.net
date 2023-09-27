use tracing_tokio::writer::HtmlWriter;

fn tracing_init() {
    use tracing::Level;
    use tracing_subscriber::{filter::FilterFn, fmt::format::FmtSpan, prelude::*};

    let fmt_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_span_events(FmtSpan::FULL)
        // .map_writer(|w| move || HtmlWriter::new(w()))
        .with_filter(FilterFn::new(|metadata| {
            if metadata.target() == "tracing_tokio" {
                true
            } else if metadata.target() == "tokio::task" && metadata.name() == "runtime.spawn" {
                true
            } else {
                false
            }
        }));
    tracing_subscriber::registry().with(fmt_layer).init();
}

#[tokio::main]
async fn main() {
    // we will fill this in later!
    tracing_init();

    tokio::spawn(async {
        tracing::info!("step 1");

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        tracing::info!("step 2");
    })
    .await
    .expect("joining task failed");
}
