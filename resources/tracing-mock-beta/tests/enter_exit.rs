use tracing::subscriber::with_default;
use tracing_mock::{expect, subscriber};

#[test]
fn enter_exit_span() {
    let span = expect::span().named("life-universe-everything");
    let (subscriber, handle) = subscriber::mock()
        .new_span(
            span.clone()
                .with_fields(expect::field("answer").with_value(&42_u64)),
        )
        .enter(&span)
        .exit(&span)
        .enter(&span)
        .exit(&span)
        .drop_span(&span)
        .run_with_handle();

    with_default(subscriber, || {
        tracing_mock_beta::enter_exit(42);
    });

    handle.assert_finished();
}
