use serde::ser;
use serde_bytes::Bytes;

pub const UUID: &str = "UUID";

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Uuid([u8; 16]);

impl From<[u8; 16]> for Uuid {
    fn from(val: [u8; 16]) -> Self {
        Self(val)
    }
}

impl ser::Serialize for Uuid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer 
    {
        serializer.serialize_newtype_struct(UUID, Bytes::new(&self.0))    
    }
}

// TODO: optional conversion to external type uuid::Uuid;