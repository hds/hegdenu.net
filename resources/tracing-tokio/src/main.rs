use std::time::Duration;

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::builder().parse_lossy("tracing_tokio=info"))
        .init();

    tracing::info!("step 1");

    tokio::time::sleep(Duration::from_millis(100)).await;

    tracing::info!("step 2");
}
