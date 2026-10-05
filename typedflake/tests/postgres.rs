#![cfg(feature = "postgres")]

//! Round trips through a real PostgreSQL server with `tokio-postgres`.
//!
//! Set `TYPEDFLAKE_TEST_POSTGRES_URL` to run these tests; without it they are
//! skipped.

use tokio_postgres::{Client, NoTls};
use typedflake::typedflake;

#[typedflake(epoch = "2025-01-01")]
#[derive(typedflake::Postgres)]
pub struct UserId(i64);

#[typedflake(epoch = "2025-01-01", bits(timestamp = 32, node = 5, sequence = 8))]
#[derive(typedflake::Postgres)]
pub struct ReducedId(i64);

async fn connect() -> Option<Client> {
    let url = std::env::var("TYPEDFLAKE_TEST_POSTGRES_URL").unwrap_or_default();
    if url.is_empty() {
        eprintln!("skipped: TYPEDFLAKE_TEST_POSTGRES_URL is not set");
        return None;
    }
    let (client, connection) = tokio_postgres::connect(&url, NoTls)
        .await
        .expect("failed to connect to the test database");
    tokio::spawn(connection);

    client
        .batch_execute(
            "CREATE TEMPORARY TABLE users (
                id BIGINT PRIMARY KEY,
                name TEXT NOT NULL,
                referrer BIGINT
            )",
        )
        .await
        .unwrap();
    Some(client)
}

fn user_id(raw: i64) -> UserId {
    UserId::try_from(raw).unwrap()
}

#[tokio::test]
async fn ids_bind_and_read_back_without_casts() {
    let Some(client) = connect().await else {
        return;
    };
    let generator = UserId::generator(17).unwrap();
    let alice = generator.generate().unwrap();
    let bob = generator.generate().unwrap();
    let name = "Alice";

    client
        .execute(
            "INSERT INTO users (id, name) VALUES ($1, $2)",
            &[&alice, &name],
        )
        .await
        .unwrap();
    client
        .execute(
            "INSERT INTO users (id, name, referrer) VALUES ($1, $2, $3)",
            &[&bob, &"Bob", &Some(alice)],
        )
        .await
        .unwrap();

    let row = client
        .query_one("SELECT id, referrer FROM users WHERE id = $1", &[&bob])
        .await
        .unwrap();
    let id: UserId = row.try_get("id").unwrap();
    let referrer: Option<UserId> = row.try_get("referrer").unwrap();
    assert_eq!(id, bob);
    assert_eq!(referrer, Some(alice));

    let row = client
        .query_one("SELECT referrer FROM users WHERE id = $1", &[&alice])
        .await
        .unwrap();
    assert_eq!(row.get::<_, Option<UserId>>("referrer"), None);
}

#[tokio::test]
async fn limits_round_trip() {
    let Some(client) = connect().await else {
        return;
    };

    for raw in [0, 1, i64::MAX] {
        let row = client
            .query_one("SELECT $1::BIGINT", &[&user_id(raw)])
            .await
            .unwrap();
        assert_eq!(row.get::<_, UserId>(0).get(), raw);
    }
}

#[tokio::test]
async fn arrays_bind_and_read_back() {
    let Some(client) = connect().await else {
        return;
    };
    let ids = vec![user_id(1), user_id(2), user_id(3)];

    for (id, name) in ids.iter().zip(["a", "b", "c"]) {
        client
            .execute("INSERT INTO users (id, name) VALUES ($1, $2)", &[id, &name])
            .await
            .unwrap();
    }

    let rows = client
        .query(
            "SELECT name FROM users WHERE id = ANY($1) ORDER BY id",
            &[&&ids[..2]],
        )
        .await
        .unwrap();
    let names: Vec<String> = rows.iter().map(|row| row.get(0)).collect();
    assert_eq!(names, ["a", "b"]);

    let row = client
        .query_one("SELECT array_agg(id ORDER BY id) FROM users", &[])
        .await
        .unwrap();
    assert_eq!(row.get::<_, Vec<UserId>>(0), ids);
}

#[tokio::test]
async fn decoding_rejects_values_that_are_not_valid_ids() {
    use std::error::Error;

    let Some(client) = connect().await else {
        return;
    };

    let row = client.query_one("SELECT -1::BIGINT", &[]).await.unwrap();
    let negative = row.try_get::<_, UserId>(0).unwrap_err();
    assert_eq!(negative.source().unwrap().to_string(), "ID -1 is negative");
    assert_eq!(row.get::<_, i64>(0), -1);

    let row = client
        .query_one("SELECT $1::BIGINT", &[&(1_i64 << 45)])
        .await
        .unwrap();
    let reserved = row.try_get::<_, ReducedId>(0).unwrap_err();
    assert!(
        reserved
            .source()
            .unwrap()
            .to_string()
            .contains("sets reserved bits"),
        "{reserved:?}"
    );

    let row = client
        .query_one("SELECT $1::BIGINT", &[&((1_i64 << 45) - 1)])
        .await
        .unwrap();
    assert_eq!(row.get::<_, ReducedId>(0).get(), (1 << 45) - 1);

    let row = client
        .query_one("SELECT ARRAY[1, -1]::BIGINT[]", &[])
        .await
        .unwrap();
    assert!(row.try_get::<_, Vec<UserId>>(0).is_err());
}

#[tokio::test]
async fn ids_are_only_compatible_with_bigint() {
    let Some(client) = connect().await else {
        return;
    };

    let row = client
        .query_one("SELECT '1'::TEXT AS id, 1::INT AS small", &[])
        .await
        .unwrap();
    assert!(row.try_get::<_, UserId>("id").is_err());
    assert!(row.try_get::<_, UserId>("small").is_err());

    let wrong_parameter = client.query_one("SELECT $1::TEXT", &[&user_id(1)]).await;
    assert!(wrong_parameter.is_err());
}
