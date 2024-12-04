pub fn hello(name: &str) {
    tracing::info!(key = "value", "Hello, {}!", name);
}
