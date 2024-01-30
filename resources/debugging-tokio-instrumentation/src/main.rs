#[tokio::main]
async fn main() {
    colored::control::set_override(true);

    use tracing_subscriber::prelude::*;
    tracing_subscriber::registry()
        .with(ari_subscriber::layer())
        .init();

    tokio::spawn(async move {
        tracing::info!(fun = true, "pre-yield");
        tokio::task::yield_now().await;
    })
    .await
    .unwrap();
}
