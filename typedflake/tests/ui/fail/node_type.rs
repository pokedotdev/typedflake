use typedflake::typedflake;

#[derive(Debug, Clone, Copy)]
struct NotANode {
    worker: u8,
}

#[typedflake(epoch = "2025-01-01", node = NotANode)]
struct UserId(i64);

#[typedflake(epoch = "2025-01-01", node = u16)]
struct OtherIntegerNode(i64);

fn main() {}
