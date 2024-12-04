pub fn hello(name: &str) {
    tracing::info!(name, "Hello!");
}

pub fn enter_exit(val: u64) {
    let span = tracing::info_span!("life-universe-everything", answer = val);

    {
        let _guard = span.enter();
    }

    {
        let _guard = span.enter();
    }
}
