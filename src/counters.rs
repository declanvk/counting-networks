//! Concrete implementations of shared counter using counting networks implenented in this crate.

use std::cell::Cell;

use networks::BitonicNetwork;

struct CountingBucket {
    value: Cell<usize>,
}

impl CountingBucket {
    fn new(starting_value: usize) -> Self {
        CountingBucket {
            value: Cell::new(starting_value),
        }
    }

    fn get(&self) -> usize {
        self.value.get()
    }

    fn inc(&self, increment: usize) {
        self.value.set(self.value.get() + increment);
    }
}

/// Output sequential values without duplicates or skips.
pub trait Counter {
    /// Retrieve value from counter and update internal state.
    fn next(&self) -> usize;
}

/// Concrete counter based on [BitonicNetwork](super::networks::BitonicNetwork).
pub struct BitonicCountingNetwork(BitonicNetwork<CountingBucket>);

impl BitonicCountingNetwork {
    /// Create a new counter with specified width.
    ///
    /// Choice of width will not effect output of the counter, but higher values will ensure
    /// less contention among threads while accessing the counter at the cost of more memory.
    ///
    /// # Examples
    ///
    /// ```
    /// use counting_networks::counters::{Counter, BitonicCountingNetwork};
    ///
    /// let counter = BitonicCountingNetwork::new(8);
    /// 
    /// assert_eq!(counter.next(), 0);
    /// ```
    pub fn new(width: usize) -> Self {
        let outputs = (0..width)
            .map(|v| CountingBucket::new(v))
            .collect::<Vec<_>>();
        BitonicCountingNetwork(BitonicNetwork::new(outputs))
    }

    /// Returns the output width of the internal bitonic network.
    ///
    /// # Examples
    ///
    /// ```
    /// use counting_networks::counters::BitonicCountingNetwork;
    ///
    /// let counter = BitonicCountingNetwork::new(8);
    /// 
    /// assert_eq!(counter.width(), 8);
    /// ```
    pub fn width(&self) -> usize {
        self.0.width()
    }
}

impl Counter for BitonicCountingNetwork {
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
