use std::{future::Future, task::Poll};

#[tokio::main]
async fn main() {
    // Function that returns ready immediately
    println!("Before ready().await");
    ready().await;
    println!("After ready().await");

    // Function that returns pending immediately
    println!("Before pending().await");
    pending().await;
    println!("After pending().await");
}

fn ready() -> Ready {
    Ready {}
}

struct Ready;

impl Future for Ready {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        println!("Ready: poll()");
        Poll::Ready(())
    }
}

fn pending() -> impl Future<Output = ()> {
    Pending {}
}

struct Pending;

impl Future for Pending {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        println!("Pending: poll()");
        Poll::Pending
    }
}
