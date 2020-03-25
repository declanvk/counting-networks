use counting_networks::networks::BitonicNetwork;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

const NETWORK_WIDTH_RANGE: &[usize] = &[2, 8, 32, 128, 512];

fn construct_bitonic_network(c: &mut Criterion) {
    let mut group = c.benchmark_group("construct_bitonic_network");

    for width in NETWORK_WIDTH_RANGE {
        group.bench_with_input(BenchmarkId::from_parameter(width), &width, |b, &width| {
            b.iter(|| BitonicNetwork::new((0..*width).collect()))
        });
    }
    group.finish();
}

criterion_group!(construct_network_benches, construct_bitonic_network,);
criterion_main!(construct_network_benches);
