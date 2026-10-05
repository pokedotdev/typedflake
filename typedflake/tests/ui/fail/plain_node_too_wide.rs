use typedflake::typedflake;

#[typedflake(epoch = "2025-01-01", bits(timestamp = 20, node = 33, sequence = 10))]
struct UserId(i64);

fn main() {}
