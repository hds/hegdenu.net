use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex, MutexGuard},
    task::{Context, Poll},
    time::Duration,
};

use tokio::time::Sleep;

fn main() {
    let body = async {
        let data = Arc::new(Mutex::new(0_u64));

        tokio::spawn(sleepless_exclusive_value(Arc::clone(&data)));
        exclusive_value_sleep(Arc::clone(&data))
            .await
            .expect("failed to perform operation");
    };

    return tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build runtime")
        .block_on(body);
}

fn exclusive_value_sleep(
    data: Arc<Mutex<u64>>,
) -> impl Future<Output = Result<(), DataAccessError>> {
    ExclusiveValueSleep::Init { data }
}

enum ExclusiveValueSleep<'a> {
    Init {
        data: Arc<Mutex<u64>>,
    },
    Sleep {
        data: Arc<Mutex<u64>>,
        sleep: Pin<Box<Sleep>>,
        guard: MutexGuard<'a, u64>,
    },
    AfterSleep {
        _data: Arc<Mutex<u64>>,
        guard: MutexGuard<'a, u64>,
    },
    Done,
}

impl<'a> Future for ExclusiveValueSleep<'a> {
    type Output = Result<(), DataAccessError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let this = std::mem::replace(&mut *self, Self::Done);
            let (new, return_value) = match this {
                Self::Init { data } => {
                    let guard = unsafe {
                        // SAFETY: We will hold on to the Arc containing the mutex as long
                        //         as we hold onto the guard.
                        std::mem::transmute::<MutexGuard<'_, u64>, MutexGuard<'static, u64>>(
                            data.lock().map_err(|_| DataAccessError {})?,
                        )
                    };
                    println!("existing value: {}", *guard);

                    (
                        Self::Sleep {
                            data,
                            sleep: Box::pin(tokio::time::sleep(Duration::from_millis(10))),
                            guard: guard,
                        },
                        None,
                    )
                }
                Self::Sleep {
                    data,
                    mut sleep,
                    guard,
                } => {
                    let pinned_sleep = Pin::new(&mut sleep);
                    match pinned_sleep.poll(cx) {
                        Poll::Pending => (Self::Sleep { data, sleep, guard }, Some(Poll::Pending)),
                        Poll::Ready(_) => (Self::AfterSleep { _data: data, guard }, None),
                    }
                }
                Self::AfterSleep { _data, guard } => {
                    println!("new value: {}", *guard);
                    (Self::Done, Some(Poll::Ready(Ok(()))))
                }
                Self::Done => panic!("Please stop polling me!"),
            };
            _ = std::mem::replace(&mut *self, new);
            if let Some(poll) = return_value {
                return poll;
            }
        }
    }
}

async fn sleepless_exclusive_value(data: Arc<Mutex<u64>>) -> Result<(), DataAccessError> {
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
