#[tokio::main]
async fn main() {
    let section = "Two step async function";
    dbg!(
        section,
        async_await::triple_add(1, 2, 3).await,
        manual_future::triple_add(1, 2, 3).await,
        manual_future::triple_add2(1, 2, 3).await,
    );
}

mod async_await {
    pub async fn triple_add(x: u64, y: u64, z: u64) -> u64 {
        let c = x + y;

        tokio::task::yield_now().await;

        c + z
    }
}

mod manual_future {
    use std::{future::Future, pin::Pin, task::Poll};

    pub fn triple_add(x: u64, y: u64, z: u64) -> TripleAdd {
        TripleAdd::Init { x, y, z }
    }

    #[derive(Debug)]
    pub enum TripleAdd {
        Init { x: u64, y: u64, z: u64 },
        Step1 { c: u64, z: u64 },
        Done,
    }

    impl Future for TripleAdd {
        type Output = u64;

        fn poll(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> Poll<Self::Output> {
            match *self {
                Self::Init { x, y, z } => {
                    *self = Self::Step1 { c: x + y, z };
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
                Self::Step1 { c, z } => {
                    *self = Self::Done;
                    Poll::Ready(c + z)
                }
                Self::Done => panic!("Please stop polling me!"),
            }
        }
    }

    pub fn triple_add2(x: u64, y: u64, z: u64) -> TripleAdd2 {
        TripleAdd2::Init { x, y, z }
    }

    // The `YieldNow` future has beencopied from `yield_now.rs`
    // and a `new()` constructor has been added.
    #[derive(Debug)]
    pub struct YieldNow {
        yielded: bool,
    }

    impl YieldNow {
        pub fn new() -> Self {
            YieldNow { yielded: false }
        }
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

    #[derive(Debug)]
    pub enum TripleAdd2 {
        Init {
            x: u64,
            y: u64,
            z: u64,
        },
        Step1 {
            yield_now: Pin<Box<YieldNow>>,
            c: u64,
            z: u64,
        },
        Done,
    }

    impl Future for TripleAdd2 {
        type Output = u64;

        fn poll(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> Poll<Self::Output> {
            loop {
                match *self {
                    Self::Init { x, y, z } => {
                        *self = Self::Step1 {
                            yield_now: Box::pin(YieldNow::new()),
                            c: x + y,
                            z,
                        };
                    }
                    Self::Step1 {
                        ref mut yield_now,
                        c,
                        z,
                    } => match yield_now.as_mut().poll(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(_) => {
                            *self = Self::Done;
                            return Poll::Ready(c + z);
                        }
                    },
                    Self::Done => panic!("Please stop polling me!"),
                }
            }
        }
    }
}
