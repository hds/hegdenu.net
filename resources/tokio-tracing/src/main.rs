use std::time::Duration;

use opentelemetry::global;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn setup_tracing() -> Result<(), Box<dyn std::error::Error>> {
    let fmt_layer = tracing_subscriber::fmt::layer();

    // Allows you to pass along context (i.e., trace IDs) across services
    global::set_text_map_propagator(opentelemetry_jaeger::Propagator::new());
    // Sets up the machinery needed to export data to Jaeger
    // There are other OTel crates that provide pipelines for the vendors
    // mentioned earlier.
    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_service_name("tokio-tracing")
        .with_endpoint("sync-z2.ad.here.com:6831")
        .install_simple()?;

    // Create a tracing layer with the configured tracer
    let opentelemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    // The SubscriberExt and SubscriberInitExt traits are needed to extend the
    // Registry to accept `opentelemetry (the OpenTelemetryLayer type).
    tracing_subscriber::registry()
        .with(opentelemetry)
        // Continue logging to stdout
        .with(fmt_layer)
        .try_init()?;

    Ok(())
}

fn main() {
    setup_tracing().expect("failed to set up tracing subscribers");

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to create runtime");

    runtime.block_on(async {
        _ = tokio::task::Builder::new()
            .name("hello")
            .spawn(async {
                println!("Hello, world!");
                tracing::info!("Hello, world!");
            })
            .unwrap()
            .await;
    });

    runtime.block_on(async {
        tokio::time::sleep(Duration::from_secs(5)).await;
    });

    global::shutdown_tracer_provider(); // sending remaining spans
}
