//! This crate knows `typedflake` only as `tf`. Every macro must still expand
//! to paths that resolve.

use tf::{Alphabet, TypedNode, typedflake};

#[derive(Debug, Clone, Copy, PartialEq, Eq, TypedNode)]
pub struct AppNode {
    #[node(bits = 5)]
    pub worker: u8,
    #[node(bits = 5)]
    pub process: u8,
}

#[typedflake(epoch = "2025-01-01")]
#[derive(tf::Serde, tf::SqlxPostgres, tf::Postgres)]
pub struct UserId(i64);

#[typedflake(epoch = "2025-01-01", node = AppNode)]
pub struct TypedId(i64);

#[typedflake(epoch = "2025-01-01", alphabet = Alphabet::BASE62)]
#[derive(tf::SerdeEncoded)]
pub struct LinkId(i64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macros_expand_under_a_renamed_dependency() {
        let id = UserId::generator(17).unwrap().generate().unwrap();
        assert_eq!(id.parts().node, 17);
        assert_eq!(id.to_string().parse::<UserId>().unwrap(), id);

        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(serde_json::from_str::<UserId>(&json).unwrap(), id);

        let node = AppNode {
            worker: 3,
            process: 4,
        };
        let typed = TypedId::generator(node).unwrap().generate().unwrap();
        assert_eq!(typed.parts().node, node);

        let link = LinkId::try_from(62_i64).unwrap();
        assert_eq!(link.encode().as_str(), "00000000010");
        assert_eq!(LinkId::decode(&link.encode()).unwrap(), link);
        let json = serde_json::to_string(&link).unwrap();
        assert_eq!(json, r#""00000000010""#);
        assert_eq!(serde_json::from_str::<LinkId>(&json).unwrap(), link);
    }
}
