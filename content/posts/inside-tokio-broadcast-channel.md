+++
title = "inside tokio: broadcast channel"
slug = "inside-tokio-broadcast-channel"
author = "hds"
date = "2024-09-30"
+++

Tokio is the most popular async runtime for Rust. It has (as of the time of writing) [830 contributors](https://github.com/tokio-rs/tokio/graphs/contributors) on GitHub, but there are probably only a very small number of people who understand all the deep internals of the `tokio` crate. I'm definitely not in that list myself. But there are some bits of Tokio that I understand reasonably well, and since the best way to really understand something is to teach it to someone else, I thought I'd try to explain one of those bits in a post.

As a quick aside, I've heard good things about [Jon Gjengset](https://github.com/jonhoo)'s marathon [Decrusting the tokio crate](https://www.youtube.com/watch?v=o2ob8zkeq2s) video, if you want to get into some of the details. I haven't watched it myself, mostly because I'm intimidated by starting a 3.5 hour long video.

This post is going to get into how Tokio's [Broadcast channel](https://docs.rs/tokio/1.40.0/tokio/sync/broadcast/index.html) works. We're looking at the implementation as of the latest Tokio release, [v1.40.0](https://github.com/tokio-rs/tokio/releases/tag/tokio-1.40.0), so all the links to code and documentation will be for that version.

## broadcast channel

Tokio's broadcast channel is a multi-producer, multi-consumer queue, where each sent value is seen by all consumers. One important detail is that each `Receiver` (consumer) will only see messages which were sent **after** it was created - since new `Receiver` instances can be created from any existing `Receiver` or `Sender` (producer). I'll reiterate this point later on.

![A diagram of a broadcast channel with senders and receivers. The 3 senders each send a message (m1, m2, m3) and the 2 receivers each receive all three messages.](/img/inside-tokio-broadcast-channel/broadcast-channel-concept.svg)

Each `Receiver` will see each message in order.

### bounded

Tokio's broadcast channel is a bounded channel, it has a maximum number of messages that it can store (or "buffer"), this is [configured when the channel is created](https://docs.rs/tokio/1.40.0/tokio/sync/broadcast/fn.channel.html).

Any bounded channel needs a way to handle the situation where it is full and another message is sent. This broadcast channel handles being full by overwriting the oldest element in the channel. This allows well-behaving receivers to continue receiving new messages, while receivers which are too slow will receive the `N` latest messages when they finally catch up (for a channel with capacity `N`). A slow receiver which misses messages in this way is said to have "lagged".

This is different to Tokio's multi-producing, single-consumer (MPSC) channel. When a bounded MPSC channel is full, it will asynchronously wait for capacity to be freed up before inserting a new message - thus the single consumer will never lose messages.

### usage

Let's look at a usage example for a broadcast channel, modified from the Tokio docs:

```rust
use tokio::sync::broadcast;

#[tokio::main]
async fn main() {
    let (tx, mut rx1) = broadcast::channel(16);

    tx.send(10).unwrap();
    let mut rx2 = tx.subscribe();

    tokio::spawn(async move {
        assert_eq!(rx1.recv().await.unwrap(), 10);
        assert_eq!(rx1.recv().await.unwrap(), 20);
    });

    tokio::spawn(async move {
        assert_eq!(rx2.recv().await.unwrap(), 20);
    });

    tx.send(20).unwrap();
}
```

Let's quickly go through each part of this code.

We create a broadcast channel with a capacity of 16 slots. This returns a single sender (`tx`) and a single receiver (`rx1`). 

We send a first message (`10`) into the channel. After that, we create a new receiver (`rx2`). This order is important because it means that the second receiver will **not** receive the message sent before its creation.

Now we spawn 2 tasks. Before looking at the contents of the tasks, let's skip to the bottom and check what is happening while these tasks get spawn. Here a second message (`20`) is sent into the channel. That means that (ignoring concurrent interleaving), we now have 2 messages in our channel, `10` and `20`.

Back to our tasks. We move the receiver `rx1` into the first task and call `recv().await` on it twice. Upon unwrapping, these calls resolve to both messages which were sent into the channel. The second task has the receiver `rx2` moved into it. Here we only call `recv().await` once and upon unwrapping, we get only the second message `20`. This is as expected because receivers only "see" messages which were sent after their creation.

## structure

Let's start to look at the implementation of the broadcast channel. That's why we're here after all! All this code is in [`sync/broadcast.rs`](https://github.com/tokio-rs/tokio/blob/tokio-1.40.0/tokio/src/sync/broadcast.rs).

We'll start by looking at the struct definition for the [`Sender`](https://github.com/tokio-rs/tokio/blob/tokio-1.40.0/tokio/src/sync/broadcast.rs#L164) and [`Receiver`](https://github.com/tokio-rs/tokio/blob/tokio-1.40.0/tokio/src/sync/broadcast.rs#L204) halves that we've already seen.

### sender

The sender is the simpler of the two.

```rust
pub struct Sender<T> {
    shared: Arc<Shared<T>>,
}
```

The sender struct is just a wrapper around a shared reference to some struct called `Shared`. The shared reference is an `Arc` (Atomic Reference Count), so it's a thread safe smart pointer. Now we know that all senders are equal, there is no difference between them.

### receiver

The receiver is slightly more complex.

```rust
pub struct Receiver<T> {
    shared: Arc<Shared<T>>,
    next: u64,
}
```

The receiver also has the shared reference to `Shared`, but it also has a `u64` index to the next element it will read. This is what differentiates different receivers.

You'll notice that the `Sender` and the `Receiver` are both generic in `T`. This is a fancy way of saying that they can be created to work with different data types for the message, where `T` is that type. If you aren't familiar with generics in Rust, then I'd suggest you have a look at the [10.1 Generic Data Types](https://doc.rust-lang.org/book/ch10-01-syntax.html) chapter in the Rust Book. There are also many other resources available if you search online.

### shared

So it turns out that our very simple diagram above, isn't too far from the truth. Just that the "channel" is called [`Shared`](https://github.com/tokio-rs/tokio/blob/tokio-1.40.0/tokio/src/sync/broadcast.rs#L304) in the implementation. Let's have a look at that shared struct, which isn't public and so there isn't any mention of it on docs.rs.

```rust
/// Data shared between senders and receivers.
struct Shared<T> {
    /// slots in the channel.
    buffer: Box<[RwLock<Slot<T>>]>,

    /// Mask a position -> index.
    mask: usize,

    /// Tail of the queue. Includes the rx wait list.
    tail: Mutex<Tail>,

    /// Number of outstanding Sender handles.
    num_tx: AtomicUsize,
}
```

Our shared state is also generic in `T` and contains 4 members:
- `buffer` is where the actual messages in the broadcast channel are stored. It's a boxed array containing something called a `Slot` (more on that later) wrapped in a [`RwLock`](https://doc.rust-lang.org/std/sync/struct.RwLock.html).
- `mask` is how the buffer loops around in a ring.
- `tail` points to the tail of the queue - the last element to have been inserted - wrapped in a (standard library) [`Mutex`](https://doc.rust-lang.org/std/sync/struct.Mutex.html).
- `num_tx` keeps track of how many `Sender` instances currently exist.

Let's update our view of the broadcast channel.

![Broadcast channel internals. A "stack" of senders and receivers are connected to a single shared object. The receivers also hold a value `next`. The shared object holds the values `buffer` - represented as 8 slots, `mask`, `tail` - represented as an object pointing to a slot, and `num_tx`.](/img/inside-tokio-broadcast-channel/broadcast-channel-impl-1.svg)

### slot

The actual storage for the channel is in a struct called [`Slot`](https://github.com/tokio-rs/tokio/blob/tokio-1.40.0/tokio/src/sync/broadcast.rs#L334) (generic in `T`). Remember that each slot is wrapped in a `RwLock`. Now let's look at what else that slot contains.

```rust
/// Slot in the buffer.
struct Slot<T> {
    /// Remaining number of receivers that are expected to see this value.
    ///
    /// When this goes to zero, the value is released.
    ///
    /// An atomic is used as it is mutated concurrently with the slot read lock
    /// acquired.
    rem: AtomicUsize,

    /// Uniquely identifies the `send` stored in the slot.
    pos: u64,

    /// The value being broadcast.
    ///
    /// The value is set by `send` when the write lock is held. When a reader
    /// drops, `rem` is decremented. When it hits zero, the value is dropped.
    val: UnsafeCell<Option<T>>,
}
```

We've got 3 members here:
- `rem` - a count of the receivers that haven't seen this message.
- `pos` - the position that identifies the slot (different to its index). This matches up with the `next` value stored in the [receiver](#receiver).
- `val` - the actual value being transmitted through the broadcast channel.

The value in `val` is wrapped up in an [`UnsafeCell`](https://doc.rust-lang.org/std/cell/struct.UnsafeCell.html). This may seem odd, since each slot is already wrapped in a `RwLock`. We'll look at why this is necessary later.

Let's come up with a way to visually represent a slot. This involves packing information into a small space - which is usually a bad idea - but it will allow us to follow a sequence of operations on a broadcast channel, so it should be helpful. We're going to make our channel concrete right now and show a `Slot<char>` where the messages sent through the broadcast channel are single characters.

![Slot internals. For a slot with a value, we see the value, as well as the position and the remaining receivers. For a slot without a value, the value field is blank and the remaining receivers is 0, the position is still present.](/img/inside-tokio-broadcast-channel/broadcast-channel-slot-rep.svg)

We've got 2 cases for a slot.

In the first case, the slot holds a value (`val` is `Some(char)`). We see the value present (top) as well as the position (bottom left) and the remaining receivers (bottom right).

In the second case, the slot holds no value (`val` is `None`). There is no value present (top), but the position is still there (bottom left) and the remaining receivers is present (bottom right), but it will always be 0 in this case.

### tail

Now let's look at this [`Tail`](https://github.com/tokio-rs/tokio/blob/tokio-1.40.0/tokio/src/sync/broadcast.rs#L319) struct.

```rust
/// Next position to write a value.
struct Tail {
    /// Next position to write to.
    pos: u64,

    /// Number of active receivers.
    rx_cnt: usize,

    /// True if the channel is closed.
    closed: bool,

    /// Receivers waiting for a value.
    waiters: LinkedList<Waiter, <Waiter as linked_list::Link>::Target>,
}
```

There are 4 members in the tail (some of these descriptions are taken directly from the comments in the code):
- `pos` - the next position in the channel that will be written to.
- `rx_cnt` - the number of active receivers.
- `closed` - whether or not the channel is closed.
- `waiters` - a doubly linked list of receivers that are waiting for a value.

Let's try and represent this in a way that we can include in our diagrams.

![Tail internals. The tail contains a closed flag (top), list of waiters (middle), position (bottom left), and receiver count for the whole channel (bottom right).](/img/inside-tokio-broadcast-channel/broadcast-channel-tail-rep.svg)

Here we've thrown everything into a box. The position and receiver count are bottom left and right respectively so that they match the placement of the similar values in our slot representation. The closed flag is up the top (in text) and we will represent the linked list of waiters with a little box for each item in the list.

### position

Now we can have a look at how everything fits together at execution time, but first, let's clarify what the broadcast channel means by position.

The position is a 64-bit value which always increases. Of course, the position can't increase forever as it's using a fixed width representation (64 bits), a wrapping add is used to increment the position, so it will wrap around to 0 again once it reaches 2<sup>64</sup>. Why this is OK and how this value is used to access the actual ring buffer is based on carefully sizing the buffer - the capacity of the channel.

The broadcast channel has a buffer which is an array which is given a fixed size when the channel is created. There is an [important caveat in the Tokio documentation](https://docs.rs/tokio/1.40.0/tokio/sync/broadcast/fn.channel.html) regarding this size. To create a channel, we call `broadcast::channel(capacity)`, but the docs say:

> **Note**: The actual capacity may be greater than the provided `capacity`.

What actually happens is that capacity of a broadcast channel is always a power of 2 which is greater than or equal to the requested capacity. If you request a capacity of 4, you'll get 4 (2<sup>2</sup> is 4 of course). If you request a capacity of 12, you'll instead get 16 (2<sup>4</sup>). The position can then be mapped to an index into the buffer by masking off all but the bottom N bits for an actual buffer size of 2<sup>N</sup>.

Let's look at how this would work if we had a 4-bit position (an imaginary `u4` type) and a real capacity of 4 (2<sup>2</sup>). We have N=2, so we need to mask all but the bottom 2 bits (binary `0011`). The first 8 values for our position would then look like the following:

| position | mask   | index  |
|----------|--------|--------|
| `0000`   | `0011` | `0000` |
| `0001`   | `0011` | `0001` |
| `0010`   | `0011` | `0010` |
| `0011`   | `0011` | `0011` |
| `0100`   | `0011` | `0000` |
| `0101`   | `0011` | `0001` |
| `0110`   | `0011` | `0010` |
| `0111`   | `0011` | `0011` |

We see that the position keeps increasing all the way up to 7 (`0111`) and could really reach 15 with our 4-bit representation, however once the index reaches 3 (`0011`) the next index goes back to 0 (`0000`).

The capacity of the channel is limited to half of `usize::MAX`, which should be no greater than half of `u64::MAX` on supported platforms. This is important for the position wrapping, and we'll discuss it later.

## runtime

Let's now look at how a broadcast channel operates at runtime. We'll start with the creation of the channel.

### channel creation

We're going to make things a little more concrete now by specifying a channel with capacity 4 that will transmit `char`s (so `T = char`).

```rust
use tokio::sync::broadcast;

let (tx1, mut rx1) = broadcast::channel::<char>(4);
```

Before we go on, one small detail. We're working with a small sized channel, and to make some of the values that we need to show easier to represent, we're going to pretend for this exercise that all [position](#position) values are stored in a 4-bit representation (that imaginary `u4`) instead of the 64-bit representation (`u64`) that is really used. This will allow us to explore wrapping without using huge numbers.

Let's introduce our full visual representation of the channel and get familiar with reading it. I've done it this way because there's a lot of state held and a table holding all this data would be too large.

![Visual representation of the newly created broadcast channel.](/img/inside-tokio-broadcast-channel/broadcast-channel-runtime-0-creation.svg)

We've got one sender `tx1` and one receiver `rx1`, the receiver has its `next` position set to 0.

The channel has a capacity of 4, so the `mask` (shown in binary) accepts only 2 bits. The number of senders `num_tx` is 1.

Our buffer has 4 slots. None of them have values so all they all have remaining receivers `rem` set to 0, but they do each have an initial position recorded.

The position recorded is the slot's index in the buffer (0 to 3) minus the size of the buffer (4), performed as a wrapping subtraction. So the initial position `pos` values are 12, 13, 14, and 15 for indices 0, 1, 2, and 3 respectively.

Finally, the tail indicates that the channel is not closed, it points to the slot at the tail of the buffer (position 0, which is index 0) and stores that there is currently 1 receiver active. The waiter list is empty of course.

### send 'a'

Let's send our first message through the channel.

```rust
tx1.send('a').unwrap();
```

This function is synchronous, not `async`, so when it returns, the channel has been updated. Let's check the new state. The values that have changed since the previous state are marked in <span style="color:#f00">red</span>.

![Visual representation of the broadcast channel after sending 1 message.](/img/inside-tokio-broadcast-channel/broadcast-channel-runtime-1-send-a.svg)

Here we can see that the slot in buffer index 0 has been updated. It now has a value (`'a'`), its position is now 0, and there is 1 remaining receiver (the only one) which hasn't yet seen this value.

The tail has also been updated, its position field now points to position 1 (index 1 in the buffer).

Since we're here for all the gory details, let's go through the code that brought us to this state. Here is the implementation of the [`Sender::send`](https://github.com/tokio-rs/tokio/blob/tokio-1.40.0/tokio/src/sync/broadcast.rs#L586) function.

```rust
pub fn send(&self, value: T) -> Result<usize, SendError<T>> {
    let mut tail = self.shared.tail.lock();

    if tail.rx_cnt == 0 {
        return Err(SendError(value));
    }

    // Position to write into
    let pos = tail.pos;
    let rem = tail.rx_cnt;
    let idx = (pos & self.shared.mask as u64) as usize;

    // Update the tail position
    tail.pos = tail.pos.wrapping_add(1);

    // Get the slot
    let mut slot = self.shared.buffer[idx].write().unwrap();

    // Track the position
    slot.pos = pos;

    // Set remaining receivers
    slot.rem.with_mut(|v| *v = rem);

    // Write the value
    slot.val = UnsafeCell::new(Some(value));

    // Release the slot lock before notifying the receivers.
    drop(slot);

    // Notify and release the mutex. This must happen after the slot lock is
    // released, otherwise the writer lock bit could be cleared while another
    // thread is in the critical section.
    self.shared.notify_rx(tail);

    Ok(rem)
}
```

First we lock the tail (it's in a mutex). This means that only 1 sender can be in this function at once.

Then we check whether there are any receivers. If there are no receivers, we return an error. The error contains the value which the caller attempted to send, so it can be recovered.

Next we collect information about the position we're going to write into. This comes from the tail. We store the position (0) and the current number of receivers (1). Finally, we calculate the index by masking the position with the mask (it will still be 0, but a position of 4 would also be index 0).

The tail position is then updated by performing a wrapping add, which will result in 1 in this case.

Now we lock the slot at index 0 for writing. There may be receivers reading from this slot (they are soon to be lagging), but there should be no contention for the write lock as another sender would have to obtain a lock on the tail mutex first.

Now we set the new position for this slot (0, it was previously 12).

Then we set the remaining receivers which haven't seen this value. Since the value is just being written, the remaining receivers is the total number of receivers, 1.

After that we write the value itself `'a'`.

Then we drop the write lock on the slot.

After dropping the slot write guard, we notify any waiting receivers. There is a comment on the order of releasing the slot lock and notifying the receivers, we'll dig into that later.

Finally, the current number of receivers is returned to the caller.

Wow, that was quite a bit all written out like that. But I think that it's fairly straight forward to follow.

### subscribe a second receiver

A broadcast channel receiver can't be cloned from another one like a sender can be. Instead either `Sender::subscribe()` or `Receiver::resubscribe()` needs to be used to create a new receiver. The important point here is that the newly subscribed receiver will only see messages sent after its creation, it won't receive any backlog of messages that may exist.

```rust
let mut rx2 = tx1.subscribe();
```

This will make one small change to our shared channel struct, let's have a look at that. Once again, the changes since the last state are shown in <span style="color:#f00">red</span>.

![Visual representation of the broadcast channel after subscribing a second receiver.](/img/inside-tokio-broadcast-channel/broadcast-channel-runtime-2-receiver-subscribe.svg)

The first thing we notice here is that we have 2 receivers now. The second receiver has a `next` position of 1, which is taken from the tail position. This is what makes the new receiver see only messages sent after it is created.

We can also see that the remaining receivers `rx_cnt` value has been incremented to 2.


