use typedflake::typedflake;

#[typedflake(epoch = "2025-01-01")]
#[derive(Clone)]
struct DerivesClone(i64);

#[typedflake(epoch = "2025-01-01")]
#[derive(Default, std::fmt::Debug)]
struct DerivesDebug(i64);

#[typedflake(epoch = "2025-01-01")]
#[derive(PartialEq, Eq)]
struct DerivesEq(i64);

#[typedflake(epoch = "2025-01-01")]
#[derive(serde::Serialize)]
struct DerivesSerialize(i64);

#[typedflake(epoch = "2025-01-01")]
#[derive(Deserialize)]
struct DerivesDeserialize(i64);

#[typedflake(epoch = "2025-01-01")]
#[derive(sqlx::Type)]
struct DerivesSqlxType(i64);

#[typedflake(epoch = "2025-01-01")]
#[derive(ToSql, FromSql)]
struct DerivesToSql(i64);

#[typedflake(epoch = "2025-01-01")]
#[repr(C)]
struct HasRepr(i64);

#[typedflake(epoch = "2025-01-01")]
#[typedflake(epoch = "2025-01-01")]
struct DeclaredTwice(i64);

fn main() {}
