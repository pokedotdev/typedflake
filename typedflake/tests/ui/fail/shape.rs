use typedflake::typedflake;

#[typedflake(epoch = "2025-01-01")]
struct Named {
    value: i64,
}

#[typedflake(epoch = "2025-01-01")]
struct TwoFields(i64, i64);

#[typedflake(epoch = "2025-01-01")]
struct Unit;

#[typedflake(epoch = "2025-01-01")]
struct WrongInteger(u32);

#[typedflake(epoch = "2025-01-01")]
struct Generic<T>(T);

#[typedflake(epoch = "2025-01-01")]
struct PublicField(pub i64);

#[typedflake(epoch = "2025-01-01")]
enum NotAStruct {
    Variant(i64),
}

fn main() {}
