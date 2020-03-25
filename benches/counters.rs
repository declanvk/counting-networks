use core::sync::atomic::{AtomicUsize, Ordering};
use counting_networks::counters::{BitonicCountingNetwork, Counter};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::sync::Arc;
use std::thread;

fn atomic_count_to(counter: Arc<AtomicUsize>, num_threads: usize, max_counter_value: usize) {
    let mut thread_handles = Vec::new();

    for _ in 0..num_threads {
        let counter_copy = counter.clone();
        let handle = thread::spawn(move || {
            let mut last_value = 0;
            while last_value < max_counter_value {
                last_value = counter_copy.fetch_add(1, Ordering::SeqCst);
            }
        });

        thread_handles.push(handle);
    }

    thread_handles.into_iter().for_each(|handle| {
        handle.join().unwrap();
    });
}

fn network_count_to(
    counter: Arc<BitonicCountingNetwork>,
    num_threads: usize,
    max_counter_value: usize,
) {
    let mut thread_handles = Vec::new();

    for _ in 0..num_threads {
        let counter_copy = counter.clone();
        let handle = thread::spawn(move || {
            let mut last_value = 0;
            while last_value < max_counter_value {
                last_value = counter_copy.next();
            }
        });

        thread_handles.push(handle);
    }

    thread_handles.into_iter().for_each(|handle| {
        handle.join().unwrap();
    });
}

const COMMON_COUNTER_LIMIT: usize = 10_000;
const LIMIT_UNIT: usize = 1000;
const COUNTER_LIMIT_RANGE: &[usize] = &[
    LIMIT_UNIT,
    2 * LIMIT_UNIT,
    4 * LIMIT_UNIT,
    8 * LIMIT_UNIT,
    16 * LIMIT_UNIT,
];

pub fn counter_vary_contention(c: &mut Criterion) {
    let mut group = c.benchmark_group("counter_vary_contention");
    let atomic_counter = Arc::new(AtomicUsize::new(0));
    let network_counter = Arc::new(BitonicCountingNetwork::new(
        num_cpus::get().next_power_of_two(),
    ));

    for num_threads in 1..=num_cpus::get() {
        group.bench_with_input(
            BenchmarkId::new("atomic", num_threads),
            &num_threads,
            |b, &num_threads| {
                b.iter(|| {
                    atomic_count_to(
                        Arc::clone(&atomic_counter),
                        num_threads,
                        black_box(COMMON_COUNTER_LIMIT),
                    )
                })
            },
        );
        group.bench_with_input(
            BenchmarkId::new("network", num_threads),
            &num_threads,
            |b, &num_threads| {
                b.iter(|| {
                    network_count_to(
                        Arc::clone(&network_counter),
                        num_threads,
                        black_box(COMMON_COUNTER_LIMIT),
                    )
                })
            },
        );
    }
    group.finish();
}

pub fn counter_low_contention_vary_limit(c: &mut Criterion) {
    let mut group = c.benchmark_group("counter_low_contention_vary_limit");
    let atomic_counter = Arc::new(AtomicUsize::new(0));
    let network_counter = Arc::new(BitonicCountingNetwork::new(
        num_cpus::get().next_power_of_two(),
    ));

    for counter_limit in COUNTER_LIMIT_RANGE {
        group.throughput(Throughput::Bytes(*counter_limit as u64));
        group.bench_with_input(
            BenchmarkId::new("atomic", counter_limit),
            &counter_limit,
            |b, &counter_limit| {
                b.iter(|| {
                    atomic_count_to(Arc::clone(&atomic_counter), 1, black_box(*counter_limit))
                })
            },
        );
        group.bench_with_input(
            BenchmarkId::new("network", counter_limit),
            &counter_limit,
            |b, &counter_limit| {
                b.iter(|| {
                    network_count_to(Arc::clone(&network_counter), 1, black_box(*counter_limit))
                })
            },
        );
    }
    group.finish();
}

pub fn counter_high_contention_vary_limit(c: &mut Criterion) {
    let mut group = c.benchmark_group("counter_high_contention_vary_limit");
    let atomic_counter = Arc::new(AtomicUsize::new(0));
    let network_counter = Arc::new(BitonicCountingNetwork::new(
        num_cpus::get().next_power_of_two(),
    ));
    let cpu_count = num_cpus::get();

    for counter_limit in COUNTER_LIMIT_RANGE {
        group.throughput(Throughput::Bytes(*counter_limit as u64));
        group.bench_with_input(
            BenchmarkId::new("atomic", counter_limit),
            &counter_limit,
            |b, &counter_limit| {
                b.iter(|| {
                    atomic_count_to(
                        Arc::clone(&atomic_counter),
                        cpu_count,
                        black_box(*counter_limit),
                    )
                })
            },
        );
        group.bench_with_input(
            BenchmarkId::new("network", counter_limit),
            &counter_limit,
            |b, &counter_limit| {
                b.iter(|| {
                    network_count_to(
                        Arc::clone(&network_counter),
                        cpu_count,
                        black_box(*counter_limit),
                    )
                })
            },
        );
    }
    group.finish();
}

criterion_group!(
    counter_benches,
    counter_vary_contention,
    counter_low_contention_vary_limit,
    counter_high_contention_vary_limit,
);
criterion_main!(counter_benches);
