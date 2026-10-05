use typedflake::{Alphabet, typedflake};

#[typedflake(epoch = "2025-01-01")]
#[derive(typedflake::SerdeEncoded)]
struct NoAlphabet(i64);

#[typedflake(epoch = "2025-01-01", alphabet = Alphabet::BASE62)]
#[derive(typedflake::Serde, typedflake::SerdeEncoded)]
struct BothSerdeDerives(i64);

fn main() {}
