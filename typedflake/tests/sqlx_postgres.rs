#![cfg(feature = "sqlx-postgres")]

//! Round trips through a real PostgreSQL server.
//!
//! Set `TYPEDFLAKE_TEST_POSTGRES_URL` to run these tests; without it they are
//! skipped. Each test uses temporary tables on a single connection.

use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use typedflake::typedflake;

#[typedflake(epoch = "2025-01-01")]
#[derive(typedflake::SqlxPostgres)]
pub struct UserId(i64);

#[typedflake(epoch = "2025-01-01", bits(timestamp = 32, node = 5, sequence = 8))]
#[derive(typedflake::SqlxPostgres)]
pub struct ReducedId(i64);

#[derive(Debug, PartialEq, sqlx::FromRow)]
struct User {
    id: UserId,
    name: String,
    referrer: Option<UserId>,
}

async fn connect() -> Option<PgPool> {
    let url = std::env::var("TYPEDFLAKE_TEST_POSTGRES_URL").unwrap_or_default();
    if url.is_empty() {
        eprintln!("skipped: TYPEDFLAKE_TEST_POSTGRES_URL is not set");
        return None;
    }
    // Temporary tables belong to one session, so the pool holds one connection.
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .expect("failed to connect to the test database");
    sqlx::query(
        "CREATE TEMPORARY TABLE users (
            id BIGINT PRIMARY KEY,
            name TEXT NOT NULL,
            referrer BIGINT
        )",
    )
    .execute(&pool)
    .await
    .unwrap();
    Some(pool)
}

fn user_id(raw: i64) -> UserId {
    UserId::try_from(raw).unwrap()
}

#[tokio::test]
async fn ids_bind_and_read_back_without_casts() {
    let Some(pool) = connect().await else { return };
    let generator = UserId::generator(17).unwrap();
    let alice = generator.generate().unwrap();
    let bob = generator.generate().unwrap();

    sqlx::query("INSERT INTO users (id, name) VALUES ($1, $2)")
        .bind(alice)
        .bind("Alice")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users (id, name, referrer) VALUES ($1, $2, $3)")
        .bind(bob)
        .bind("Bob")
        .bind(Some(alice))
        .execute(&pool)
        .await
        .unwrap();

    let users: Vec<User> = sqlx::query_as("SELECT id, name, referrer FROM users ORDER BY id")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(
        users,
        [
            User {
                id: alice,
                name: "Alice".to_owned(),
                referrer: None,
            },
            User {
                id: bob,
                name: "Bob".to_owned(),
                referrer: Some(alice),
            },
        ]
    );

    let found: UserId = sqlx::query_scalar("SELECT id FROM users WHERE id = $1")
        .bind(bob)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(found, bob);
}

#[tokio::test]
async fn limits_round_trip() {
    let Some(pool) = connect().await else { return };

    for raw in [0, 1, i64::MAX] {
        let echoed: UserId = sqlx::query_scalar("SELECT $1::BIGINT")
            .bind(user_id(raw))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(echoed.get(), raw);
    }
}

#[tokio::test]
async fn arrays_bind_and_read_back() {
    let Some(pool) = connect().await else { return };
    let ids = vec![user_id(1), user_id(2), user_id(3)];

    for (id, name) in ids.iter().zip(["a", "b", "c"]) {
        sqlx::query("INSERT INTO users (id, name) VALUES ($1, $2)")
            .bind(*id)
            .bind(name)
            .execute(&pool)
            .await
            .unwrap();
    }

    let names: Vec<String> =
        sqlx::query_scalar("SELECT name FROM users WHERE id = ANY($1) ORDER BY id")
            .bind(&ids[..2])
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(names, ["a", "b"]);

    let collected: Vec<UserId> = sqlx::query_scalar("SELECT array_agg(id ORDER BY id) FROM users")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(collected, ids);

    let echoed: Vec<UserId> = sqlx::query_scalar("SELECT $1::BIGINT[]")
        .bind(ids.clone())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(echoed, ids);
}

#[tokio::test]
async fn decoding_rejects_values_that_are_not_valid_ids() {
    let Some(pool) = connect().await else { return };

    let negative = sqlx::query_scalar::<_, UserId>("SELECT -1::BIGINT")
        .fetch_one(&pool)
        .await
        .unwrap_err();
    assert!(
        negative.to_string().contains("ID -1 is negative"),
        "{negative}"
    );

    let reserved = sqlx::query_scalar::<_, ReducedId>("SELECT $1::BIGINT")
        .bind(1_i64 << 45)
        .fetch_one(&pool)
        .await
        .unwrap_err();
    assert!(
        reserved.to_string().contains("sets reserved bits"),
        "{reserved}"
    );

    let largest: ReducedId = sqlx::query_scalar("SELECT $1::BIGINT")
        .bind((1_i64 << 45) - 1)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(largest.get(), (1 << 45) - 1);

    let in_array = sqlx::query_scalar::<_, Vec<UserId>>("SELECT ARRAY[1, -1]::BIGINT[]")
        .fetch_one(&pool)
        .await
        .unwrap_err();
    assert!(
        in_array.to_string().contains("ID -1 is negative"),
        "{in_array}"
    );

    let row = sqlx::query("SELECT -7::BIGINT AS id")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(row.try_get::<UserId, _>("id").is_err());
    assert_eq!(row.try_get::<i64, _>("id").unwrap(), -7);
}

#[tokio::test]
async fn ids_are_only_compatible_with_bigint() {
    let Some(pool) = connect().await else { return };

    let row = sqlx::query("SELECT '1'::TEXT AS id, 1::INT AS small")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(row.try_get::<UserId, _>("id").is_err());
    assert!(row.try_get::<UserId, _>("small").is_err());
}
