#[tokio::main(flavor = "current_thread")]
async fn main() {
    let data = Arc::new(Mutex::new(0_u64));

    tokio::spawn(yieldless_mutex_access(Arc::clone(&data)));
    hold_mutex_guard(Arc::clone(&data))
        .await
        .expect("failed to perform operation");
}

use std::sync::{Arc, Mutex};

async fn hold_mutex_guard(data: Arc<Mutex<u64>>) -> Result<(), DataAccessError> {
    let mut guard = data.lock().map_err(|_| DataAccessError {})?;
    println!("existing value: {}", *guard);

    tokio::task::yield_now().await;

    *guard = *guard + 1;
    println!("new value: {}", *guard);

    Ok(())
}

async fn yieldless_mutex_access(data: Arc<Mutex<u64>>) -> Result<(), DataAccessError> {
    let mut guard = data.lock().map_err(|_| DataAccessError {})?;
    println!("existing value: {}", *guard);

    *guard = *guard + 1;
    println!("new value: {}", *guard);

    Ok(())
}

use std::{error::Error, fmt};

#[derive(Debug)]
struct DataAccessError {}
impl fmt::Display for DataAccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "there was an error accessing the shared data")
    }
}
impl Error for DataAccessError {}
