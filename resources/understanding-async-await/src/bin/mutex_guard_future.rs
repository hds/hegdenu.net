use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

fn main() {
    let body = async {
        let data = Arc::new(Mutex::new(0_u64));

        tokio::spawn(yieldless_mutex_access(Arc::clone(&data)));
        hold_mutex_guard(Arc::clone(&data))
            .await
            .expect("failed to perform operation");
    };

    return tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build runtime")
        .block_on(body);
}

fn hold_mutex_guard(data: Arc<Mutex<u64>>) -> impl Future<Output = Result<(), DataAccessError>> {
    HoldMutexGuard::Init { data }
}

use std::sync::{Arc, Mutex, MutexGuard};

enum HoldMutexGuard<'a> {
    Init {
        data: Arc<Mutex<u64>>,
    },
    Yielded {
        guard: MutexGuard<'a, u64>,
        _data: Arc<Mutex<u64>>,
    },
    Done,
}

impl<'a> Future for HoldMutexGuard<'a> {
    type Output = Result<(), DataAccessError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let state = &mut *self;
        match state {
            Self::Init { data } => {
                let guard = unsafe {
                    // SAFETY: We will hold on to the Arc containing the mutex as long
                    //         as we hold onto the guard.
                    std::mem::transmute::<MutexGuard<'_, u64>, MutexGuard<'static, u64>>(
                        data.lock().map_err(|_| DataAccessError {})?,
                    )
                };
                println!("existing value: {}", *guard);

                cx.waker().wake_by_ref();
                *state = Self::Yielded {
                    guard: guard,
                    _data: Arc::clone(data),
                };

                Poll::Pending
            }
            Self::Yielded { guard, _data } => {
                println!("new value: {}", *guard);

                *state = Self::Done;

                Poll::Ready(Ok(()))
            }
            Self::Done => panic!("Please stop polling me!"),
        }
    }
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
