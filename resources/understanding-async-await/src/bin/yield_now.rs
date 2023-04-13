#[tokio::main]
async fn main() {
    // Function that yields back to the runtime immediately
    dbg!(async_await::yield_now().await);
    dbg!(manual_future::yield_now().await);
}

mod async_await {
    pub async fn yield_now() {
        tokio::task::yield_now().await
    }
}

mod manual_future {
    use std::{future::Future, task::Poll};

    pub fn yield_now() -> YieldNow {
        YieldNow { yielded: false }
    }

    #[derive(Debug)]
    pub struct YieldNow {
        yielded: bool,
    }

    impl Future for YieldNow {
        type Output = ();

        fn poll(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> Poll<Self::Output> {
            if self.yielded == true {
                return Poll::Ready(());
            }

            self.yielded = true;

            cx.waker().wake_by_ref();

            Poll::Pending
        }
    }
}
