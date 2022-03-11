use std::convert::TryFrom;

use serde::de;
use serde::ser;
// use serde_bytes::ByteBuf;
use serde_bytes::Bytes;

use crate::__constants::UUID;
use crate::error::Error;
use crate::fixed_width::UUID_WIDTH;

/// A universally unique identifier as defined by RFC-4122 in section 4.1.2
///
/// encoding code = 0x98,
/// category = fixed, width = 16,
/// label="UUID as defined in section 4.1.2 of RFC-4122"
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Uuid([u8; UUID_WIDTH]);

impl Uuid {
    /// Consume the wrapper into the inner bytes
    pub fn into_inner(self) -> [u8; UUID_WIDTH] {
        self.0
    }
}

impl From<[u8; UUID_WIDTH]> for Uuid {
    fn from(val: [u8; UUID_WIDTH]) -> Self {
        Self(val)
    }
}

impl From<Uuid> for [u8; UUID_WIDTH] {
    fn from(val: Uuid) -> Self {
        val.0
    }
}

impl TryFrom<&[u8]> for Uuid {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != UUID_WIDTH {
            return Err(Error::InvalidLength);
        }

        let mut buf = [0u8; UUID_WIDTH];
        buf.copy_from_slice(&value[..UUID_WIDTH]);
        Ok(Self(buf))
    }
}

impl ser::Serialize for Uuid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_newtype_struct(UUID, Bytes::new(&self.0))
    }
}

struct Visitor {}

impl<'de> de::Visitor<'de> for Visitor {
    type Value = Uuid;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("struct Uuid")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Uuid::try_from(v).map_err(|err| de::Error::custom(err.to_string()))
    }
}

impl<'de> de::Deserialize<'de> for Uuid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_newtype_struct(UUID, Visitor {})
    }
}
