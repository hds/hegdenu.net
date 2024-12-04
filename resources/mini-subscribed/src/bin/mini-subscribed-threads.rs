use std::time::Duration;

use console_subscriber::ServerParts;
use tracing_subscriber::{prelude::*, EnvFilter};

fn main() {
    let (console_layer, server) = console_subscriber::ConsoleLayer::builder()
        .retention(Duration::from_secs(10))
        //.enable_self_trace(true)
        .build();

    tracing::info!("main-thread: Hello, world!");

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<String>();

    let join_handle = std::thread::Builder::new()
        .name("aggregator".into())
        .spawn(move || {
            let subscriber = tracing_subscriber::fmt()
                .with_env_filter(
                    EnvFilter::builder()
                        .parse_lossy("console_subscriber=debug,mini_subscribed=info,info"),
                )
                .finish();
            let _subscriber_guard = tracing::subscriber::set_default(subscriber);
            tracing::info!("test wot?");

            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .max_blocking_threads(1)
                .build()
                .expect("aggregator-thread: failed to build tokio runtime");

            runtime.block_on(async move {
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

                tracing::info!("aggregator-thread: Waiting for completion");
                match shutdown_rx.await {
                    Ok(msg) => tracing::info!("aggregator-thread: shutdown signal received: {msg}"),
                    Err(err) => tracing::info!("aggregator-thread: shutdown error: {err:?}"),
                };
                tracing::info!("aggregator-thread: Completion received");
                aggregator_handle.abort();
                serve_handle.abort();
            });

            tracing::info!("aggregator-thread: Shutting down runtime in 30 seconds.");
            runtime.shutdown_timeout(Duration::from_secs(30));
        })
        .expect("aggregator-thread: failed to spawn");

    tracing_subscriber::registry()
        .with(console_layer)
        //.with(tracing_subscriber::fmt::layer())
        .init();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .expect("main-thread: failed to build tokio runtime");
    runtime.block_on(async move {
        _ = tokio::task::Builder::default()
            .name("Dornröschen")
            .spawn(async {
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

                tracing::info!("Completed, waiting 30 seconds for aggregator to empty.");
                tokio::time::sleep(Duration::from_secs(30)).await;

                match shutdown_tx.send("moo".to_string()) {
                    Ok(_) => tracing::info!("main-thread: sent shutdown signal."),
                    Err(err) => {
                        tracing::info!("main-thread: failed to send shutdown signal: {err:?}")
                    }
                }
            })
            .expect("main-thread: failed to spawn Dornröschen")
            .await;
    });

    tracing::info!("main-thread: Shutting down runtime in 30 seconds.");
    runtime.shutdown_timeout(Duration::from_secs(30));
    match join_handle.join() {
        Ok(_) => tracing::info!("main-thread: joined aggregator thread"),
        Err(err) => tracing::info!("main-thread: failed to join aggregator thread: {err:?}"),
    }
}
