#![cfg(loom)]

use counting_networks::counters::{BitonicCountingNetwork, Counter};
use loom::thread;
use std::sync::Arc;

#[test]
fn loom_test_counter_no_duplicates() {
    const COUNT_COUNT: usize = 4;
    loom::model(|| {
        // strange panic at 'assertion failed: self.threads.len() < self.max()' if this
        // is set to `num_cpu::get()`
        let thread_count = num_cpus::get() - 1;

        let counter = Arc::new(BitonicCountingNetwork::new(
            thread_count.next_power_of_two(),
        ));
        let mut thread_handles = Vec::new();

        for _ in 0..thread_count {
            let counter_copy = counter.clone();
            let handle = thread::spawn(move || {
                let mut values = Vec::new();
                for _ in 0..COUNT_COUNT {
                    let value = counter_copy.next();
                    values.push(value)
                }
                values
            });

            thread_handles.push(handle);
        }

        let mut results: Vec<usize> =
            thread_handles
                .into_iter()
                .fold(Vec::new(), |mut container, handle| {
                    container.extend(handle.join().unwrap());

                    container
                });
        results.sort();
        assert_eq!(
            results,
            (0..(thread_count * COUNT_COUNT)).collect::<Vec<_>>()
        );
    })
}
