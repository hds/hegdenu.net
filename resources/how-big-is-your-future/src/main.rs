fn how_big_is_that_future<F: std::future::Future>(_fut: F) -> usize {
    std::mem::size_of::<F>()
}

async fn nothing() {
}

async fn huge() {
    let mut a = [0_u8; 20_000];
    nothing().await;
    for (idx, item) in a.iter_mut().enumerate() {
        *item = (idx % 256) as u8;
    }
}

async fn innocent() {
    huge().await;
}

async fn not_so_innocent() {
    Box::pin(huge()).await;
}

fn main() {
    println!("huge: {}", how_big_is_that_future(huge()));
    println!("innocent: {}", how_big_is_that_future(innocent()));
    println!("not so innocent: {}", how_big_is_that_future(not_so_innocent()));
}
