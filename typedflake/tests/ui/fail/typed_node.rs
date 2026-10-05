use typedflake::TypedNode;

#[derive(Clone, Copy, TypedNode)]
struct MissingBits {
    #[node(bits = 5)]
    worker: u8,
    process: u8,
}

#[derive(Clone, Copy, TypedNode)]
struct WideField {
    #[node(bits = 5)]
    worker: u64,
}

#[derive(Clone, Copy, TypedNode)]
struct ZeroBits {
    #[node(bits = 0)]
    worker: u8,
}

#[derive(Clone, Copy, TypedNode)]
struct BitsExceedType {
    #[node(bits = 9)]
    worker: u8,
}

#[derive(Clone, Copy, TypedNode)]
struct UnknownOption {
    #[node(width = 5)]
    worker: u8,
}

#[derive(Clone, Copy, TypedNode)]
struct DuplicateBits {
    #[node(bits = 5)]
    #[node(bits = 5)]
    worker: u8,
}

#[derive(Clone, Copy, TypedNode)]
struct TooWide {
    #[node(bits = 32)]
    high: u32,
    #[node(bits = 31)]
    low: u32,
}

#[derive(Clone, Copy, TypedNode)]
struct Tuple(u8, u8);

#[derive(Clone, Copy, TypedNode)]
struct Empty {}

#[derive(Clone, Copy, TypedNode)]
struct Generic<T> {
    #[node(bits = 5)]
    worker: T,
}

#[derive(Clone, Copy, TypedNode)]
enum NotAStruct {
    Variant,
}

fn main() {}
