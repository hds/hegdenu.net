#[tokio::main]
async fn main() {
    // Hello, World async function
    async_await::hello("world").await;
    manual_future::hello("world").await;
}

mod async_await {
    pub async fn hello(name: &'static str) {
        println!("hello, {name}!");
    }
}

mod manual_future {
    use std::{future::Future, task::Poll};

    pub fn hello(name: &'static str) -> impl Future<Output = ()> {
        Hello::Init { name }
    }

    #[derive(Debug)]
    enum Hello {
        Init { name: &'static str },
        Done,
    }

    impl Future for Hello {
        type Output = ();

        fn poll(
            mut self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> Poll<Self::Output> {
            match *self {
                Hello::Init { name } => println!("hello, {name}!"),
                Hello::Done => panic!("Please stop polling me!"),
            };

            *self = Hello::Done;
            Poll::Ready(())
        }
    }
}
