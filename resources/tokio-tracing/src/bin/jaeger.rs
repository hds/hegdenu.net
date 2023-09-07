use std::time::Duration;

use opentelemetry::global;
use opentelemetry::trace::Tracer;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    global::set_text_map_propagator(opentelemetry_jaeger::Propagator::new());
    let tracer = opentelemetry_jaeger::new_agent_pipeline().install_simple()?;

    tracer.in_span("doing_work", |_cx| {
        // Traced app logic here...
        println!("Traced app logic here...");
        std::thread::sleep(Duration::from_secs(2));
    });

    global::shutdown_tracer_provider(); // sending remaining spans

    Ok(())
}
