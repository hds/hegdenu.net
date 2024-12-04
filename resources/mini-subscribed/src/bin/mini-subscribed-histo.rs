use hdrhistogram::serialization::{Serializer, V2Serializer};
use rand::Rng;
use std::{collections::VecDeque, time::Duration};

fn work() {
    let mut rng = rand::thread_rng();

    let max = Duration::from_secs(1).as_nanos() as u64;
    let mut keep = VecDeque::with_capacity(5);

    for _ in 0..50 {
        let mut histogram = hdrhistogram::Histogram::<u64>::new_with_max(max, 2)
            .expect("failed to create histogram");
        for _ in 0..5 {
            loop {
                let elapsed =
                    (Duration::from_millis(500).as_nanos() as u64) + (rng.gen::<u16>() as u64);
                if elapsed <= max {
                    histogram
                        .record(elapsed)
                        .expect("already clamped value to max");
                    break;
                }
            }
            if keep.len() >= 5 {
                keep.pop_front();
            }
            let mut serializer = V2Serializer::new();
            let mut raw_histogram = Vec::new();
            serializer
                .serialize(&histogram, &mut raw_histogram)
                .expect("histogram failed to serialize");
            keep.push_back(raw_histogram);
        }
    }
}

fn main() {
    std::thread::sleep(Duration::from_millis(500));

    println!("histo: Hello, world!");

    work();

    std::thread::sleep(Duration::from_millis(500));
}
