use typedflake::typedflake;

#[typedflake(epoch = "2025-01-01")]
#[derive(typedflake::SqlxPostgres)]
struct SqlxId(u64);

#[typedflake(epoch = "2025-01-01")]
#[derive(typedflake::Postgres)]
struct PostgresId(u64);

fn main() {}
