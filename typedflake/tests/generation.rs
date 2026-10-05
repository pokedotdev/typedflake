//! Explicit generators against the real clock. Nothing here calls `init`.

use std::collections::HashSet;
use std::sync::Barrier;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use typedflake::{GenerateError, Generator, GeneratorError, Id, NodeError, TypedNode, typedflake};

#[derive(Debug, Clone, Copy, PartialEq, Eq, TypedNode)]
pub struct AppNode {
    #[node(bits = 5)]
    pub worker: u8,
    #[node(bits = 5)]
    pub process: u8,
}

fn unix_millis_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[test]
fn generated_ids_carry_the_node_and_the_current_time() {
    #[typedflake(epoch = "2025-01-01")]
    struct UserId(i64);

    let before = unix_millis_now();
    let generator: Generator<UserId> = UserId::generator(17).unwrap();
    let id = generator.generate().unwrap();
    let after = unix_millis_now();

    assert_eq!(id.parts().node, 17);
    assert_eq!(generator.node(), 17);
    assert!((before..=after).contains(&id.unix_millis().unwrap()));
    assert!(id.get() > 0);
}

#[test]
fn ids_from_one_generator_are_unique_and_increasing() {
    #[typedflake(epoch = "2025-01-01")]
    struct UserId(i64);

    let generator = UserId::generator(1).unwrap();
    let ids: Vec<UserId> = (0..10_000)
        .map(|_| generator.generate_blocking(Duration::from_secs(1)).unwrap())
        .collect();

    assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn typed_node_generators_take_the_declared_node() {
    #[typedflake(epoch = "2025-01-01", node = AppNode)]
    struct UserId(i64);

    let node = AppNode {
        worker: 17,
        process: 1,
    };
    let generator = UserId::generator(node).unwrap();
    let id = generator.generate().unwrap();

    assert_eq!(id.parts().node, node);
    assert_eq!(generator.node(), node);

    assert_eq!(
        UserId::generator(AppNode {
            worker: 32,
            process: 1,
        })
        .unwrap_err(),
        GeneratorError::InvalidNode(NodeError::new("worker", 32, 31))
    );
}

#[test]
fn unsigned_and_reduced_formats_generate() {
    #[typedflake(epoch = "2025-01-01")]
    struct WideId(u64);

    #[typedflake(epoch = "2025-01-01", bits(timestamp = 40, node = 4, sequence = 6))]
    struct SmallId(i64);

    let wide = WideId::generator(1023).unwrap().generate().unwrap();
    assert_eq!(wide.parts().node, 1023);

    let small = SmallId::generator(15).unwrap().generate().unwrap();
    assert_eq!(small.parts().node, 15);
    assert!(small.get() < 1 << 50);
    assert_eq!(SmallId::try_from(small.get()), Ok(small));
}

#[test]
fn concurrent_generators_for_one_node_never_collide() {
    #[typedflake(epoch = "2025-01-01")]
    struct UserId(i64);

    const THREADS: usize = 8;
    const PER_THREAD: usize = 5_000;

    let barrier = Barrier::new(THREADS);
    let ids: Vec<UserId> = thread::scope(|scope| {
        let handles: Vec<_> = (0..THREADS)
            .map(|thread| {
                let barrier = &barrier;
                scope.spawn(move || {
                    // Half the threads reuse a clone, half ask for the node again.
                    let generator = UserId::generator(7).unwrap();
                    let generator = if thread % 2 == 0 {
                        generator.clone()
                    } else {
                        generator
                    };
                    barrier.wait();
                    (0..PER_THREAD)
                        .map(|_| generator.generate_blocking(Duration::from_secs(5)).unwrap())
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().unwrap())
            .collect()
    });

    let unique: HashSet<UserId> = ids.iter().copied().collect();
    assert_eq!(ids.len(), THREADS * PER_THREAD);
    assert_eq!(unique.len(), ids.len());
}

#[test]
fn different_nodes_and_types_generate_independently() {
    #[typedflake(epoch = "2025-01-01")]
    struct UserId(i64);
    #[typedflake(epoch = "2025-01-01")]
    struct OrderId(i64);

    let user_a = UserId::generator(1).unwrap().generate().unwrap();
    let user_b = UserId::generator(2).unwrap().generate().unwrap();
    let order = OrderId::generator(1).unwrap().generate().unwrap();

    assert_ne!(user_a, user_b);
    assert_eq!(user_a.parts().node, 1);
    assert_eq!(user_b.parts().node, 2);
    assert_eq!(order.parts().node, 1);
}

#[test]
fn exhausting_a_small_sequence_fails_fast_and_blocking_waits_it_out() {
    #[typedflake(epoch = "2025-01-01", bits(timestamp = 41, node = 10, sequence = 1))]
    struct TinyId(i64);

    let generator = TinyId::generator(0).unwrap();

    // Two IDs per millisecond: a tight loop is certain to run out.
    let exhausted =
        (0..10_000).any(|_| matches!(generator.generate(), Err(GenerateError::SequenceExhausted)));
    assert!(exhausted);

    let ids: HashSet<TinyId> = (0..50)
        .map(|_| generator.generate_blocking(Duration::from_secs(1)).unwrap())
        .collect();
    assert_eq!(ids.len(), 50);
}

#[test]
fn generators_are_send_sync_and_debuggable() {
    #[typedflake(epoch = "2025-01-01")]
    struct UserId(i64);

    fn assert_send_sync<T: Send + Sync + 'static>() {}
    assert_send_sync::<Generator<UserId>>();
    assert_send_sync::<UserId>();

    let generator = UserId::generator(5).unwrap();
    let debug = format!("{generator:?}");
    assert!(
        debug.contains("UserId") && debug.contains("node: 5"),
        "{debug}"
    );
}

#[test]
fn generic_code_can_generate_through_the_trait() {
    #[typedflake(epoch = "2025-01-01")]
    struct UserId(i64);

    fn mint<I: Id>(node: I::Node) -> I {
        I::generator(node).unwrap().generate().unwrap()
    }

    let id: UserId = mint(9);
    assert_eq!(id.parts().node, 9);
}

#[test]
fn errors_describe_themselves() {
    use std::error::Error;

    #[typedflake(epoch = "2025-01-01")]
    struct UserId(i64);

    let error = UserId::generator(5_000).unwrap_err();
    assert_eq!(error.to_string(), "node does not fit the ID format");
    assert_eq!(
        error.source().unwrap().to_string(),
        "node field `node` is 5000, but its maximum is 1023"
    );
}
