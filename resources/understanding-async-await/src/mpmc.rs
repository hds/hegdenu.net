use core::fmt;
use std::error::Error;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::task::Waker;
use std::{collections::VecDeque, task::Poll};

/// Creates a new asynchronous bounded multi-producer multi-consumer channel,
/// returning the sender/receiver halves.
///
/// The channel will buffer messages up to the defined capacity. Once the
/// buffer is full, attempts to send new messages will wait until a message is
/// received from the channel. When the channel is empty, attempts to receive
/// new messages will wait until  a message is sent to the channel.
///
/// If all receivers or all senders have disconnected, the channel will be
/// closed. Subsequent attempts to send a message will return a
/// [`ChannelClosedError`]. Subsequent attempts to receive a message will drain
/// the channel and once it is empty, will also return a [`ChannelClosedError`].
pub fn channel(capacity: usize) -> (Sender, Receiver) {
    let inner = Arc::new(Mutex::new(Channel::new(capacity)));

    (Sender::new(inner.clone()), Receiver::new(inner))
}

/// Error returned when the underlying channel is closed.
///
/// This error will be returned from [`Sender::send`] or [`Receiver::recv`] if
/// the channel is closed. In the case of `recv`, the channel must also be
/// empty, otherwise the next value will be returned.
#[derive(Debug)]
pub struct ChannelClosedError {}
impl fmt::Display for ChannelClosedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "channel closed")
    }
}
impl Error for ChannelClosedError {}

/// The sending-half of the [`mpmc::channel`] type.
///
/// Messages can be sent through the channel with [`send`].
///
/// This half can be cloned to send from multiple tasks. Dropping all senders
/// will cause the channel to be closed.
///
/// [`mpmc::channel`]: fn@super::mpmc::channel
/// [`send`]: fn@Self::send
pub struct Sender {
    inner: Arc<Mutex<Channel>>,
}

impl Sender {
    fn new(inner: Arc<Mutex<Channel>>) -> Self {
        {
            match inner.lock() {
                Ok(mut guard) => guard.senders += 1,
                Err(_) => panic!("MPMC Channel has become corrupted."),
            }
        }
        Self { inner }
    }

    /// Sends a value, waiting until there is capacity.
    ///
    /// A successful send occurs when there is at least one [`Receiver`] still
    /// connected to the channel. An `Err` result means that the value will
    /// never be received, however an `Ok` result doesn't guarantee that the
    /// value will be received as all receivers may disconnect immediately
    /// after this method returns `Ok`.
    pub async fn send(&self, value: String) -> Result<(), ChannelClosedError> {
        Send {
            value,
            inner: self.inner.clone(),
        }
        .await
    }
}

impl Clone for Sender {
    fn clone(&self) -> Self {
        Self::new(self.inner.clone())
    }
}

impl Drop for Sender {
    fn drop(&mut self) {
        match self.inner.lock() {
            Ok(mut guard) => guard.dec_senders(),
            Err(_) => panic!("MPMC Channel has become corrupted."),
        }
    }
}

struct Send {
    value: String,
    inner: Arc<Mutex<Channel>>,
}

impl Future for Send {
    type Output = Result<(), ChannelClosedError>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let Ok(mut guard) = self.inner.lock() else {
            panic!("MPMC Channel has become corrupted.");
        };

        match guard.send(self.value.clone()) {
            Ok(_) => Poll::Ready(Ok(())),
            Err(ChannelSendError::Closed) => Poll::Ready(Err(ChannelClosedError {})),
            Err(ChannelSendError::Full) => {
                guard.register_sender_waker(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

/// The receiving-half of the [`mpmc::channel`] type.
///
/// Messages can be received from the channel with [`recv`].
///
/// This half can be cloned to receive from multiple tasks. Each message will
/// only be received by a single receiver. Dropping all receivers will cause
/// the channel to be closed.
///
/// [`mpmc::channel`]: fn@super::mpmc::channel
/// [`recv`]: fn@Self::recv
pub struct Receiver {
    inner: Arc<Mutex<Channel>>,
}

impl Receiver {
    fn new(inner: Arc<Mutex<Channel>>) -> Self {
        {
            match inner.lock() {
                Ok(mut guard) => guard.receivers += 1,
                Err(_) => panic!("MPMC Channel has become corrupted."),
            }
        }
        Self { inner }
    }

    /// Receives a value, waiting until one is available.
    ///
    /// Once the channel is closed (by dropping all senders), this method will
    /// continue to return the remaining values stored in the channel buffer.
    /// Once the channel is empty, this method will return
    /// [`ChannelClosedError`].
    pub async fn recv(&self) -> Result<String, ChannelClosedError> {
        Recv {
            inner: self.inner.clone(),
        }
        .await
    }
}

impl Clone for Receiver {
    fn clone(&self) -> Self {
        Self::new(self.inner.clone())
    }
}

impl Drop for Receiver {
    fn drop(&mut self) {
        match self.inner.lock() {
            Ok(mut guard) => guard.dec_receivers(),
            Err(_) => panic!("MPMC Channel has become corrupted."),
        }
    }
}

struct Recv {
    inner: Arc<Mutex<Channel>>,
}

impl Future for Recv {
    type Output = Result<String, ChannelClosedError>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let Ok(mut guard) = self.inner.lock() else {
            panic!("MPMC Channel has become corrupted.");
        };

        match guard.recv() {
            Ok(value) => Poll::Ready(Ok(value)),
            Err(ChannelRecvError::Closed) => Poll::Ready(Err(ChannelClosedError {})),
            Err(ChannelRecvError::Empty) => {
                guard.register_receiver_waker(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

/// The inner mpmc channel implementation.
///
/// This is a sync object. All methods return immediately.
struct Channel {
    /// The message buffer
    buffer: VecDeque<String>,
    /// The capacity of the channel, this many messages can be buffered before
    /// sending will error.
    capacity: usize,
    /// Indicates when the channel has been closed.
    closed: bool,

    /// The number of connected `Sender`s.
    senders: usize,
    /// The number of active `Receiver`s.
    receivers: usize,

    /// A queue of wakers for senders awaiting free capacity in the channel.
    sender_wakers: VecDeque<Waker>,
    /// A queue of wakers for receivers awaiting a new message in the channel.
    receiver_wakers: VecDeque<Waker>,
}

impl Channel {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
            closed: false,

            senders: 0,
            receivers: 0,

            sender_wakers: VecDeque::new(),
            receiver_wakers: VecDeque::new(),
        }
    }

    fn send(&mut self, value: String) -> Result<(), ChannelSendError> {
        if self.closed {
            return Err(ChannelSendError::Closed);
        }

        if self.buffer.len() < self.capacity {
            self.buffer.push_front(value);
            if let Some(waker) = self.receiver_wakers.pop_back() {
                waker.wake();
            }
            Ok(())
        } else {
            Err(ChannelSendError::Full)
        }
    }

    fn recv(&mut self) -> Result<String, ChannelRecvError> {
        match self.buffer.pop_back() {
            Some(value) => {
                if let Some(waker) = self.sender_wakers.pop_back() {
                    waker.wake();
                }
                Ok(value)
            }
            None => {
                if !self.closed {
                    Err(ChannelRecvError::Empty)
                } else {
                    Err(ChannelRecvError::Closed)
                }
            }
        }
    }

    fn register_sender_waker(&mut self, waker: Waker) {
        self.sender_wakers.push_front(waker);
    }

    fn register_receiver_waker(&mut self, waker: Waker) {
        self.receiver_wakers.push_front(waker);
    }

    fn dec_senders(&mut self) {
        self.senders -= 1;
        if self.senders == 0 {
            self.closed = true;
        }
    }

    fn dec_receivers(&mut self) {
        self.receivers -= 1;
        if self.receivers == 0 {
            self.closed = true;
        }
    }
}

#[derive(Debug)]
enum ChannelSendError {
    Full,
    Closed,
}
impl fmt::Display for ChannelSendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for ChannelSendError {}

#[derive(Debug)]
enum ChannelRecvError {
    Empty,
    Closed,
}
impl fmt::Display for ChannelRecvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for ChannelRecvError {}
