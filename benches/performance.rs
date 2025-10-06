use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use typedflake::{BitLayout, Config, Epoch};

// Create ID types for benchmarking
typedflake::id!(BenchId);

const CUSTOM_CONFIG: Config = Config::new_unchecked(BitLayout::new(42, 5, 5, 12), Epoch::DEFAULT);

typedflake::id!(CustomBenchId, CUSTOM_CONFIG);

fn benchmark_id_generation(c: &mut Criterion) {
    c.bench_function("id_generation", |b| {
        b.iter(|| {
            let id = BenchId::generate_blocking();
            black_box(id)
        })
    });
}

fn benchmark_id_generation_custom(c: &mut Criterion) {
    c.bench_function("id_generation_custom_config", |b| {
        b.iter(|| {
            let id = CustomBenchId::generate_blocking();
            black_box(id)
        })
    });
}

fn benchmark_compose_decompose(c: &mut Criterion) {
    let id = BenchId::generate_blocking();
    let (timestamp, worker_id, process_id, sequence) = id.decompose();

    c.bench_function("compose_custom_operation", |b| {
        b.iter(|| {
            let composed = BenchId::compose_custom(
                black_box(timestamp),
                black_box(worker_id),
                black_box(process_id),
                black_box(sequence),
            )
            .unwrap();
            black_box(composed)
        })
    });

    c.bench_function("decompose_operation", |b| {
        b.iter(|| {
            let components = black_box(id).decompose();
            black_box(components)
        })
    });
}

fn benchmark_component_access(c: &mut Criterion) {
    let id = BenchId::generate_blocking();

    c.bench_function("individual_timestamp_access", |b| {
        b.iter(|| {
            let timestamp = black_box(id).timestamp();
            black_box(timestamp)
        })
    });

    c.bench_function("individual_worker_id_access", |b| {
        b.iter(|| {
            let worker_id = black_box(id).worker_id();
            black_box(worker_id)
        })
    });

    c.bench_function("all_components_access", |b| {
        b.iter(|| {
            let id = black_box(id);
            let timestamp = id.timestamp();
            let worker_id = id.worker_id();
            let process_id = id.process_id();
            let sequence = id.sequence();
            black_box((timestamp, worker_id, process_id, sequence))
        })
    });

    c.bench_function("components_struct_access", |b| {
        b.iter(|| {
            let components = black_box(id).components();
            black_box(components)
        })
    });
}

fn benchmark_blocking_generation(c: &mut Criterion) {
    c.bench_function("blocking_generation", |b| {
        b.iter(|| {
            let id = BenchId::generate_blocking();
            black_box(id)
        })
    });
}

fn benchmark_conversions(c: &mut Criterion) {
    let id = BenchId::generate_blocking();
    let raw = id.as_u64();

    c.bench_function("as_u64_conversion", |b| {
        b.iter(|| {
            let raw = black_box(id).as_u64();
            black_box(raw)
        })
    });

    c.bench_function("from_u64_unchecked_conversion", |b| {
        b.iter(|| {
            let id = BenchId::from_u64_unchecked(black_box(raw));
            black_box(id)
        })
    });

    c.bench_function("try_from_u64_validated", |b| {
        b.iter(|| {
            let id = BenchId::try_from_u64(black_box(raw)).unwrap();
            black_box(id)
        })
    });
}

fn benchmark_validation(c: &mut Criterion) {
    let id = BenchId::generate_blocking();
    let (timestamp, worker_id, process_id, sequence) = id.decompose();

    c.bench_function("compose_custom_validated", |b| {
        b.iter(|| {
            let composed = BenchId::compose_custom(
                black_box(timestamp),
                black_box(worker_id),
                black_box(process_id),
                black_box(sequence),
            )
            .unwrap();
            black_box(composed)
        })
    });

    c.bench_function("compose_custom_unchecked", |b| {
        b.iter(|| {
            let composed = BenchId::compose_custom_unchecked(
                black_box(timestamp),
                black_box(worker_id),
                black_box(process_id),
                black_box(sequence),
            );
            black_box(composed)
        })
    });
}

criterion_group!(
    benches,
    benchmark_id_generation,
    benchmark_id_generation_custom,
    benchmark_compose_decompose,
    benchmark_component_access,
    benchmark_blocking_generation,
    benchmark_conversions,
    benchmark_validation
);
criterion_main!(benches);
