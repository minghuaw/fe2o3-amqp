use std::convert::TryFrom;

use serde::ser;
use serde::de;
use serde_bytes::ByteBuf;
use serde_bytes::Bytes;

use crate::error::Error;

pub const UUID: &str = "UUID";
pub const UUID_LEN: usize = 16;

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Uuid([u8; UUID_LEN]);

impl From<[u8; UUID_LEN]> for Uuid {
    fn from(val: [u8; UUID_LEN]) -> Self {
        Self(val)
    }
}

impl TryFrom<&[u8]> for Uuid {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != UUID_LEN {
            return Err(Error::InvalidLength)
        }

        let mut buf = [0u8; UUID_LEN];
        buf.copy_from_slice(&value[..UUID_LEN]);
        Ok(Self(buf))
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

struct Visitor { }

impl<'de> de::Visitor<'de> for Visitor {
    type Value = Uuid;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("struct Uuid")
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>, 
    {
        let val: ByteBuf = de::Deserialize::deserialize(deserializer)?;
        Uuid::try_from(val.as_slice())
            .map_err(|err| de::Error::custom(err.to_string()))
    }
}

impl<'de> de::Deserialize<'de> for Uuid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> 
    {
        deserializer.deserialize_newtype_struct(UUID, Visitor { })
    }
}

// TODO: optional conversion to external type uuid::Uuid;