use understanding_async_await::mpmc::{self, Receiver, Sender};

#[tokio::main]
async fn main() {
    let (sender, receiver) = mpmc::channel(1);

    let receiver_handle = tokio::spawn(receive_loop(receiver));
    let sender_handle = tokio::spawn(send_two(sender));

    _ = sender_handle.await;
    _ = receiver_handle.await;
}

async fn send_two(sender: Sender) {
    let values = vec!["A", "B"];

    for value in values {
        sender
            .send(value.into())
            .await
            .expect("the channel has closed early");
        println!("Sent: {value}");
    }
}

async fn receive_loop(receiver: Receiver) {
    loop {
        match receiver.recv().await {
            Ok(value) => println!("Received: {value}"),
            Err(_) => {
                println!("Channel closed, exiting.");
                break;
            }
        }
    }
}
