#![cfg(feature = "tokio")]

use std::collections::HashSet;
use std::time::Duration;

use typedflake::{GenerateError, Id, typedflake};

#[typedflake(epoch = "2025-01-01")]
pub struct UserId(i64);

/// Two IDs per millisecond, so waiting is exercised constantly.
#[typedflake(epoch = "2025-01-01", bits(timestamp = 41, node = 10, sequence = 1))]
pub struct TinyId(i64);

#[tokio::test]
async fn async_generation_returns_ids() {
    let generator = UserId::generator(17).unwrap();
    let id = generator.generate_async().await.unwrap();
    assert_eq!(id.parts().node, 17);
}

#[tokio::test(flavor = "current_thread")]
async fn async_generation_waits_for_capacity_without_blocking_the_runtime() {
    let generator = TinyId::generator(1).unwrap();

    // Far more IDs than fit in the milliseconds a blocked runtime would allow
    // the other tasks; every task shares the single runtime thread.
    let tasks: Vec<_> = (0..8)
        .map(|_| {
            let generator = generator.clone();
            tokio::spawn(async move {
                let mut ids = Vec::new();
                for _ in 0..25 {
                    ids.push(generator.generate_async().await.unwrap());
                }
                ids
            })
        })
        .collect();

    let mut ids = HashSet::new();
    for task in tasks {
        ids.extend(task.await.unwrap());
    }
    assert_eq!(ids.len(), 200);
}

#[tokio::test]
async fn callers_bound_the_wait_with_a_tokio_timeout() {
    let generator = TinyId::generator(2).unwrap();

    let result = tokio::time::timeout(Duration::from_secs(5), generator.generate_async()).await;
    assert!(result.unwrap().is_ok());
}

#[tokio::test]
async fn static_async_generation_reports_missing_initialization() {
    // This process never calls `typedflake::init`.
    assert!(matches!(
        UserId::generate_async().await,
        Err(GenerateError::NotInitialized)
    ));
}

#[tokio::test]
async fn async_generation_futures_are_send() {
    fn assert_send<T: Send>(value: T) -> T {
        value
    }

    let generator = UserId::generator(3).unwrap();
    let id = tokio::spawn(assert_send(async move { generator.generate_async().await }))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(id.parts().node, 3);

    async fn generic<I: Id>() -> Result<I, GenerateError> {
        I::generate_async().await
    }
    assert!(assert_send(generic::<UserId>()).await.is_err());
}
