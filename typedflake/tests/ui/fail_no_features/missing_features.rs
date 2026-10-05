use typedflake::typedflake;

#[typedflake(epoch = "2025-01-01")]
#[derive(typedflake::Serde)]
struct SerdeId(i64);

#[typedflake(epoch = "2025-01-01", alphabet = typedflake::Alphabet::BASE62)]
#[derive(typedflake::SerdeEncoded)]
struct SerdeEncodedId(i64);

#[typedflake(epoch = "2025-01-01")]
#[derive(typedflake::SqlxPostgres)]
struct SqlxId(i64);

#[typedflake(epoch = "2025-01-01")]
#[derive(typedflake::Postgres)]
struct PostgresId(i64);

fn main() {
    let _ = SerdeId::generate_async();
}
