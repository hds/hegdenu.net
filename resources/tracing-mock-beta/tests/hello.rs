use tracing::subscriber::with_default;
use tracing_mock::{expect, subscriber};

use tracing_mock_beta::hello;

#[test]
fn hello_event() {
    let (subscriber, handle) =
        subscriber::mock()
            .event(expect::event().with_fields(
                expect::msg("Hello!").and(expect::field("name").with_value(&"Hayden")),
            ))
            .run_with_handle();

    with_default(subscriber, || {
        hello("Hayden");
    });

    handle.assert_finished();
}
