#[tokio::main]
async fn main() {
    // tracing_subscriber::fmt()
    //     .with_max_level(tracing::Level::TRACE)
    //     .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
    //     .with_ansi(true)
    //     .init();

    colored::control::set_override(true);

    use tracing_subscriber::prelude::*;
    tracing_subscriber::registry()
        .with(ari_subscriber::layer())
        .init();

    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(1));

    tokio::spawn(async move {
        tracing::info!(fun = true, "pre-yield");
        tokio::task::yield_now().await;
        barrier.wait().await;
    })
    .await
    .unwrap();
}
