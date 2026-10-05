//! Run with `cargo bench -p typedflake`, optionally with `--features serde`.

use std::hint::black_box;
use std::sync::Barrier;
use std::thread;
use std::time::Instant;

use criterion::{Criterion, criterion_group, criterion_main};
use typedflake::{Parts, TypedNode, typedflake};

/// Default layout: 4096 IDs per millisecond.
#[typedflake(epoch = "2025-01-01")]
#[cfg_attr(feature = "serde", derive(typedflake::Serde))]
pub struct DefaultId(i64);

/// Four million IDs per millisecond, so the hot path never runs out of
/// sequence numbers while it is being measured.
#[typedflake(epoch = "2025-01-01", bits(timestamp = 41, node = 0, sequence = 22))]
pub struct RoomyId(i64);

#[derive(Debug, Clone, Copy, TypedNode)]
pub struct AppNode {
    #[node(bits = 5)]
    pub worker: u8,
    #[node(bits = 5)]
    pub process: u8,
}

#[typedflake(epoch = "2025-01-01", node = AppNode)]
pub struct TypedId(i64);

fn generation(c: &mut Criterion) {
    typedflake::init(0).unwrap();
    let mut group = c.benchmark_group("generate");

    group.bench_function("static", |b| b.iter(|| RoomyId::generate().unwrap()));

    let generator = RoomyId::generator(0).unwrap();
    group.bench_function("explicit", |b| b.iter(|| generator.generate().unwrap()));

    // Bound by the layout's 4096 IDs per millisecond, not by CPU time.
    group.bench_function("blocking_across_sequence_rollover", |b| {
        b.iter(|| DefaultId::generate_blocking().unwrap())
    });

    for threads in [2, 4, 8] {
        group.bench_function(format!("contended_{threads}_threads"), |b| {
            b.iter_custom(|iterations| {
                let per_thread = iterations / threads + 1;
                let barrier = Barrier::new(threads as usize + 1);
                thread::scope(|scope| {
                    for _ in 0..threads {
                        let generator = generator.clone();
                        let barrier = &barrier;
                        scope.spawn(move || {
                            barrier.wait();
                            for _ in 0..per_thread {
                                // Exhaustion is possible when every thread
                                // lands in one millisecond; it is still an
                                // answer from the generator.
                                let _ = black_box(generator.generate());
                            }
                        });
                    }
                    barrier.wait();
                    Instant::now()
                })
                .elapsed()
            });
        });
    }

    group.finish();
}

fn initialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("generator");

    group.bench_function("lookup_existing_node", |b| {
        b.iter(|| DefaultId::generator(black_box(7)).unwrap())
    });
    group.bench_function("clone", |b| {
        let generator = DefaultId::generator(7).unwrap();
        b.iter(|| generator.clone())
    });

    group.finish();
}

fn values(c: &mut Criterion) {
    let mut group = c.benchmark_group("value");
    let id = DefaultId::try_from(232_900_560_974_681_078_i64).unwrap();
    let typed = TypedId::try_from(232_900_560_974_681_078_i64).unwrap();
    let text = id.to_string();

    group.bench_function("try_from_i64", |b| {
        b.iter(|| DefaultId::try_from(black_box(id.get())).unwrap())
    });
    group.bench_function("parts", |b| b.iter(|| black_box(id).parts()));
    group.bench_function("parts_typed_node", |b| b.iter(|| black_box(typed).parts()));
    group.bench_function("from_parts", |b| {
        let parts: Parts<u32> = id.parts();
        b.iter(|| DefaultId::from_parts(black_box(parts)).unwrap())
    });
    group.bench_function("from_parts_typed_node", |b| {
        let parts = typed.parts();
        b.iter(|| TypedId::from_parts(black_box(parts)).unwrap())
    });
    group.bench_function("to_string", |b| b.iter(|| black_box(id).to_string()));
    group.bench_function("parse", |b| {
        b.iter(|| black_box(text.as_str()).parse::<DefaultId>().unwrap())
    });

    #[cfg(feature = "serde")]
    {
        let json = serde_json::to_string(&id).unwrap();
        group.bench_function("serde_json_serialize", |b| {
            b.iter(|| serde_json::to_string(&black_box(id)).unwrap())
        });
        group.bench_function("serde_json_deserialize", |b| {
            b.iter(|| serde_json::from_str::<DefaultId>(black_box(&json)).unwrap())
        });
    }

    group.finish();
}

criterion_group!(benches, generation, initialization, values);
criterion_main!(benches);
