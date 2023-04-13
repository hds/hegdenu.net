#[tokio::main]
async fn main() {
    // Simplest async function
    dbg!(async_await::add(1, 2).await);
    dbg!(manual_future::add(1, 2).await);
}

mod async_await {
    pub async fn add(x: u64, y: u64) -> u64 {
        x + y
    }
}

mod manual_future {
    use std::{future::Future, task::Poll};

    pub fn add(x: u64, y: u64) -> Add {
        Add::Init { x, y }
    }

    #[derive(Debug)]
    pub enum Add {
        Init { x: u64, y: u64 },
        Done,
    }

    impl Future for Add {
        type Output = u64;

        fn poll(
            mut self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> Poll<Self::Output> {
            let (x, y) = match *self {
                Add::Init { x, y } => (x, y),
                Add::Done => panic!("Please stop polling me!"),
            };

            *self = Add::Done;
            Poll::Ready(x + y)
        }
    }
}
