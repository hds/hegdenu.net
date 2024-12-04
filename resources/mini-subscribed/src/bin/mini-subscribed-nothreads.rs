use std::time::Duration;

use console_subscriber::ServerParts;
use tracing_subscriber::{prelude::*, EnvFilter};

fn main() {
    std::thread::sleep(Duration::from_millis(500));

    are_we_leak_yet();

    std::thread::sleep(Duration::from_millis(1500));
}

fn are_we_leak_yet() {
    let (console_layer, server) = console_subscriber::ConsoleLayer::builder()
        .retention(Duration::from_secs(10))
        .build();
    let fmt_layer = tracing_subscriber::fmt::Layer::default()
        .with_ansi(false)
        .with_filter(
            EnvFilter::builder().parse_lossy("console_subscriber=info,mini_subscribed=info,info"),
        );

    tracing_subscriber::registry()
        .with(console_layer)
        .with(fmt_layer)
        .init();

    tracing::info!("main-thread: Hello, world!");

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .expect("main-thread: failed to build tokio runtime");
    runtime.block_on(async move {
        _ = tokio::task::Builder::default()
            .name("main")
            .spawn(async {
                let ServerParts {
                    instrument_server,
                    aggregator,
                    ..
                } = server.into_parts();
                let aggregator_handle = tokio::spawn(aggregator.run());
                let router = tonic::transport::Server::builder().add_service(instrument_server);
                let serve = router.serve(std::net::SocketAddr::new(
                    std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
                    6669,
                ));
                let serve_handle = tokio::spawn(serve);

                let mut interval = tokio::time::interval(Duration::from_millis(100));

                let mut idx = 0;
                loop {
                    interval.tick().await;
                    tokio::task::Builder::default()
                        .name(&format!("child-{idx}"))
                        .spawn(async {
                            tokio::time::sleep(Duration::from_millis(500)).await;
                        })
                        .expect("main-thread: failed to spawn child task");

                    idx += 1;
                    if idx > 180 * 10 {
                        break;
                    }
                }

                tracing::info!(
                    "main-thread: completed, waiting 30 seconds for aggregator to empty."
                );
                tokio::time::sleep(Duration::from_secs(30)).await;

                aggregator_handle.abort();
                serve_handle.abort();
            })
            .expect("main-thread: failed to spawn main")
            .await;
    });

    tracing::info!("main-thread: Shutting down runtime in 30 seconds.");
    runtime.shutdown_timeout(Duration::from_secs(30));
}
