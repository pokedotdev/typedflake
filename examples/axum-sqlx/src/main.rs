//! A small HTTP service that stores typed IDs in PostgreSQL.
//!
//! ```text
//! DATABASE_URL=postgres://postgres:postgres@localhost/postgres \
//! NODE_ID=1 cargo run -p axum-sqlx
//! ```
//!
//! `UserId` is bound to queries, read from rows, parsed from the URL path, and
//! written to JSON without a single cast.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use typedflake::typedflake;

const ADDRESS: &str = "127.0.0.1:4000";

#[typedflake(epoch = "2025-01-01")]
#[derive(typedflake::Serde, typedflake::SqlxPostgres)]
pub struct UserId(i64);

#[derive(Debug, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
struct User {
    id: UserId,
    name: String,
}

#[derive(Serialize, Deserialize)]
struct CreateUser {
    name: String,
}

fn app(pool: PgPool) -> Router {
    Router::new()
        .route("/users", post(create_user))
        .route("/users/{id}", get(get_user))
        .with_state(pool)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Each running instance needs its own node number.
    let node: u32 = std::env::var("NODE_ID")?.parse()?;
    typedflake::init(node)?;

    let pool = PgPool::connect(&std::env::var("DATABASE_URL")?).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS users (id BIGINT PRIMARY KEY, name TEXT NOT NULL)")
        .execute(&pool)
        .await?;

    let listener = tokio::net::TcpListener::bind(ADDRESS).await?;
    println!("Listening on http://{ADDRESS}");
    axum::serve(listener, app(pool)).await?;
    Ok(())
}

async fn create_user(
    State(pool): State<PgPool>,
    Json(input): Json<CreateUser>,
) -> Result<(StatusCode, Json<User>), StatusCode> {
    // Waits on a Tokio timer in the rare case a millisecond's IDs run out.
    let id = UserId::generate_async()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    sqlx::query("INSERT INTO users (id, name) VALUES ($1, $2)")
        .bind(id)
        .bind(&input.name)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((
        StatusCode::CREATED,
        Json(User {
            id,
            name: input.name,
        }),
    ))
}

async fn get_user(
    State(pool): State<PgPool>,
    Path(id): Path<UserId>,
) -> Result<Json<User>, StatusCode> {
    sqlx::query_as::<_, User>("SELECT id, name FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, header};
    use http_body_util::BodyExt;
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;

    use super::*;

    /// Needs a database: set `TYPEDFLAKE_TEST_POSTGRES_URL` to run it.
    #[tokio::test]
    async fn creates_and_fetches_a_user_over_http() {
        let url = std::env::var("TYPEDFLAKE_TEST_POSTGRES_URL").unwrap_or_default();
        if url.is_empty() {
            eprintln!("skipped: TYPEDFLAKE_TEST_POSTGRES_URL is not set");
            return;
        }
        let _ = typedflake::init(1);

        // One connection, so the temporary table is visible to every query.
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        sqlx::query("CREATE TEMPORARY TABLE users (id BIGINT PRIMARY KEY, name TEXT NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();
        let app = app(pool);

        let request = Request::post("/users")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"name":"Alice"}"#))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let created: User = serde_json::from_slice(&body).unwrap();
        assert_eq!(created.name, "Alice");

        // The ID travels as a JSON string and as a path segment.
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["id"], created.id.to_string());

        let request = Request::get(format!("/users/{}", created.id))
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(serde_json::from_slice::<User>(&body).unwrap(), created);

        for (path, status) in [
            ("/users/1", StatusCode::NOT_FOUND),
            ("/users/-1", StatusCode::BAD_REQUEST),
            ("/users/abc", StatusCode::BAD_REQUEST),
        ] {
            let request = Request::get(path).body(Body::empty()).unwrap();
            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(response.status(), status, "{path}");
        }
    }
}
