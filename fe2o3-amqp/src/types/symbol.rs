use serde::Serialize;

pub const SYMBOL_MAGIC: &str = "SYMBOL";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Symbol(String);

impl Symbol {
    pub fn new(val: String) -> Self {
        Self(val)
    }
}

impl From<String> for Symbol {
    fn from(val: String) -> Self {
        Self(val)
    }
}

impl Serialize for Symbol {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_newtype_struct(SYMBOL_MAGIC, &self.0)
    }
}

// #[cfg(test)]
// mod tests{
//     use super::*;

//     #[test]
//     fn test_serialize_symbol() {
//         let symbol = Symbol("amqp".into());
//         let expected = [0xa3 as u8, 0x04, 0x61, 0x6d, 0x71, 0x70];
//         let
//     }
// }
