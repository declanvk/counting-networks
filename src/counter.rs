use std::cell::Cell;

use bitonic_network::BitonicNetwork;

pub struct CountingBucket {
    value: Cell<usize>,
}

impl CountingBucket {
    pub fn new(starting_value: usize) -> Self {
        CountingBucket {
            value: Cell::new(starting_value),
        }
    }

    pub fn get(&self) -> usize {
        self.value.get()
    }

    pub fn inc(&self, increment: usize) {
        self.value.set(self.value.get() + increment);
    }
}

pub trait Counter {
    fn new(width: usize) -> Self;
    fn width(&self) -> usize;
    fn next(&self) -> usize;
}

pub struct BitonicCountingNetwork(BitonicNetwork<CountingBucket>);

impl Counter for BitonicCountingNetwork {
    fn new(width: usize) -> Self {
        let outputs = (0..width)
            .map(|v| CountingBucket::new(v))
            .collect::<Vec<_>>();
        BitonicCountingNetwork(BitonicNetwork::new(width, outputs))
    }

    fn width(&self) -> usize {
        self.0.width()
    }

    fn next(&self) -> usize {
        let bucket = self.0.traverse();

        let output = bucket.get();
        bucket.inc(self.width());

        output
    }
}

#[cfg(test)]
mod tests {

    use std::sync::Arc;
    use std::thread;

    use super::*;

    #[test]
    fn create_counter() {
        const WIDTH: usize = 16;
        let counter = BitonicCountingNetwork::new(WIDTH);

        assert_eq!(counter.width(), WIDTH);
    }

    fn sync_only<T: Sync>(_: T) {}
    fn send_only<T: Send>(_: T) {}

    #[test]
    fn is_send() {
        send_only(BitonicCountingNetwork::new(4));
    }

    #[test]
    fn is_sync() {
        sync_only(BitonicCountingNetwork::new(4));
    }

    #[test]
    fn concurrent_counting() {
        const WIDTH: usize = 8;
        const NUM_THREADS: usize = 8;
        const NUM_COUNTS: usize = 4;

        let counter = Arc::new(BitonicCountingNetwork::new(WIDTH));
        let mut thread_handles = Vec::new();
        for _ in 0..NUM_THREADS {
            let counter_copy = counter.clone();
            let handle = thread::spawn(move || {
                let mut values = Vec::new();
                for _ in 0..NUM_COUNTS {
                    let value = counter_copy.next();
                    values.push(value)
                }
                values
            });

            thread_handles.push(handle);
        }

        let mut results: Vec<usize> = thread_handles.into_iter().fold(
            Vec::new(),
            |mut container, handle| {
                container.extend(handle.join().unwrap());

                container
            },
        );
        results.sort();
        assert_eq!(results, (0..(NUM_THREADS * NUM_COUNTS)).collect::<Vec<_>>());
    }

}
