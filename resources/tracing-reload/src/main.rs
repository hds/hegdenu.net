use std::fs::File;
use std::thread::sleep;
use std::time::Duration;

use tracing::{error, info, info_span};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{filter::LevelFilter, reload};

fn toggle_filter(filter: &mut LevelFilter) {
    if filter == &LevelFilter::ERROR {
        *filter = LevelFilter::INFO;
    } else {
        *filter = LevelFilter::ERROR;
    }
}

fn main() {
    let stdout_fmt = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .with_span_events(FmtSpan::FULL)
        .with_filter(LevelFilter::INFO); // Stdout logging INFO by default
    let (stdout_layer, stdout_reload) = reload::Layer::new(stdout_fmt);

    let log_file = File::create("output.log").expect("Couldn't open file to write traces to.");
    let file_fmt = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .with_span_events(FmtSpan::FULL)
        .with_writer(log_file)
        .with_filter(LevelFilter::ERROR); // File logging ERROR by default
    let (file_layer, file_reload) = reload::Layer::new(file_fmt);

    tracing_subscriber::registry()
        .with(stdout_layer)
        .with(file_layer)
        .init();

    info!("Set up tracing register!");

    ctrlc::set_handler(move || {
        let stdout_modified = stdout_reload.modify(|layer| toggle_filter(layer.filter_mut()));
        let file_modified = file_reload.modify(|layer| toggle_filter(layer.filter_mut()));

        match (stdout_modified, file_modified) {
            (Ok(_), Ok(_)) => info!("Successfully modified filters!"),
            (stdout_result, file_result) => error!(
                "Failed to modify at least one filer: stdout_filter={:?} file_filter:{:?}",
                stdout_result, file_result
            ),
        }
    })
    .expect("Error setting Ctrl-C handler");

    let _outer = info_span!("outer").entered();
    let inner_span = info_span!("inner");
    let mut count = 0_usize;
    loop {
        let _guard = inner_span.enter();
        for _ in 0..5 {
            info!("Tick {count}");
            count += 1;
            sleep(Duration::from_secs(1));
        }
    }
}
