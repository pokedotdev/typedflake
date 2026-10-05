use typedflake::TypedNode;

#[derive(TypedNode)]
struct NotCopy {
    #[node(bits = 5)]
    worker: u8,
}

fn main() {}
