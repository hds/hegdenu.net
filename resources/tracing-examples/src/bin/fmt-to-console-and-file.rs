fn main() {
    use tracing_subscriber::prelude::*;

    let fmt_console_filter = if cfg!(debug_assertions) {
        tracing::level_filters::LevelFilter::DEBUG
    } else {
        tracing::level_filters::LevelFilter::INFO
    };

    let log_file = std::fs::File::create("output.log").expect("couldn't open file to write logs.");
    let fmt_console = tracing_subscriber::fmt::layer().with_filter(fmt_console_filter);
    let fmt_file = tracing_subscriber::fmt::layer()
        .with_writer(log_file)
        .with_filter(tracing::level_filters::LevelFilter::DEBUG);

    tracing_subscriber::registry()
        .with(fmt_console)
        .with(fmt_file)
        .init();

    tracing::info!("This will go to console and file.");
    tracing::debug!("This will go to the file, but only to console if in debug mode.");
}
