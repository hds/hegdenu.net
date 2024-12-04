use std::time::Duration;

fn main() {
    std::thread::sleep(Duration::from_millis(500));

    console_subscriber::init();

    println!("main-thread: Hello, world!");

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("main-thread: failed to build tokio runtime");
    runtime.block_on(async move {
        _ = tokio::task::Builder::default()
            .name("main")
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

                println!("Completed, waiting 30 seconds for aggregator to empty.");
                tokio::time::sleep(Duration::from_secs(30)).await;
            })
            .expect("main-thread: failed to spawn main task")
            .await;
    });

    println!("main-thread: Shutting down runtime in 30 seconds.");
    runtime.shutdown_timeout(Duration::from_secs(30));

    std::thread::sleep(Duration::from_millis(500));
}
