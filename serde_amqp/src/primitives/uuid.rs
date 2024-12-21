use std::convert::TryFrom;
use std::fmt::LowerHex;
use std::fmt::UpperHex;

use serde::de;
use serde::ser;
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

    /// Get a reference to the inner `[u8; UUID_WIDTH]`
    pub fn as_inner(&self) -> &[u8; UUID_WIDTH] {
        &self.0
    }

    /// Get a reference to the inner `[u8; UUID_WIDTH]`
    pub fn as_inner_mut(&mut self) -> &mut [u8; UUID_WIDTH] {
        &mut self.0
    }
}

#[cfg(feature = "uuid")]
impl From<uuid::Uuid> for Uuid {
    fn from(val: uuid::Uuid) -> Self {
        Self(val.into_bytes())
    }
}

#[cfg(feature = "uuid")]
impl From<Uuid> for uuid::Uuid {
    fn from(val: Uuid) -> Self {
        Self::from_bytes(val.0)
    }
}

impl AsRef<[u8; UUID_WIDTH]> for Uuid {
    fn as_ref(&self) -> &[u8; UUID_WIDTH] {
        &self.0
    }
}

impl AsMut<[u8; UUID_WIDTH]> for Uuid {
    fn as_mut(&mut self) -> &mut [u8; UUID_WIDTH] {
        &mut self.0
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

impl de::Visitor<'_> for Visitor {
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

impl LowerHex for Uuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3],
            self.0[4], self.0[5],
            self.0[6], self.0[7],
            self.0[8], self.0[9],
            self.0[10], self.0[11], self.0[12], self.0[13], self.0[14], self.0[15],
        )
    }
}

impl UpperHex for Uuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3],
            self.0[4], self.0[5],
            self.0[6], self.0[7],
            self.0[8], self.0[9],
            self.0[10], self.0[11], self.0[12], self.0[13], self.0[14], self.0[15],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Uuid;

    #[test]
    fn test_lower_hex_formatting() {
        let uuid = [
            b'a', b'm', b'q', b'p', 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
        ];
        let uuid = Uuid::from(uuid);
        let s = format!("{:x}", uuid);
        assert_eq!(s.len(), 36);
        assert_eq!("616d7170-0506-0708-090a-0b0c0d0e0f10", s);
    }

    #[test]
    fn test_upper_hex_formatting() {
        let uuid = [
            b'a', b'm', b'q', b'p', 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
        ];
        let uuid = Uuid::from(uuid);
        let s = format!("{:X}", uuid);
        assert_eq!(s.len(), 36);
        assert_eq!("616D7170-0506-0708-090A-0B0C0D0E0F10", s);
    }
}
