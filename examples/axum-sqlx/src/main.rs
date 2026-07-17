use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use typedflake::TypedFlake;

const ADDRESS: &str = "127.0.0.1:4000";

#[derive(TypedFlake)]
pub struct UserId(u64);

#[derive(Serialize)]
struct User {
    id: UserId,
    name: String,
}

#[derive(Deserialize)]
struct CreateUser {
    name: String,
}

async fn create_pool() -> Result<SqlitePool, sqlx::Error> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;

    sqlx::query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)")
        .execute(&pool)
        .await?;

    Ok(pool)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = create_pool().await?;

    let app = Router::new()
        .route("/users", post(create_user))
        .route("/users/{id}", get(get_user))
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind(ADDRESS).await?;
    println!("Listening on http://{ADDRESS}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn create_user(
    State(pool): State<SqlitePool>,
    Json(input): Json<CreateUser>,
) -> Result<(StatusCode, Json<User>), StatusCode> {
    let id = UserId::generate();

    sqlx::query("INSERT INTO users (id, name) VALUES (?, ?)")
        .bind(id.as_u64() as i64)
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
    State(pool): State<SqlitePool>,
    Path(id): Path<u64>,
) -> Result<Json<User>, StatusCode> {
    let row = sqlx::query_as::<_, (i64, String)>("SELECT id, name FROM users WHERE id = ?")
        .bind(id as i64)
        .fetch_optional(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(User {
        id: UserId::from_u64_unchecked(row.0 as u64),
        name: row.1,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn creates_and_fetches_a_user() {
        let pool = create_pool().await.unwrap();

        let (status, Json(created)) = create_user(
            State(pool.clone()),
            Json(CreateUser {
                name: "Alice".to_owned(),
            }),
        )
        .await
        .unwrap();

        assert_eq!(status, StatusCode::CREATED);

        let Json(found) = get_user(State(pool), Path(created.id.as_u64()))
            .await
            .unwrap();

        assert_eq!(found.id, created.id);
        assert_eq!(found.name, created.name);
    }
}
