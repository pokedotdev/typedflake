#[derive(typedflake::Serde)]
struct NotDeclared(i64);

#[derive(typedflake::Serde)]
struct NotAnId {
    value: String,
}

fn main() {}
