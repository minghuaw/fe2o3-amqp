use std::fmt::{LowerHex, UpperHex};

use serde::{de, Serialize};

use super::Binary;

/// A wrapper over [`&[u8]`] that allows serialize as an AMQP Binary type and provide custom
/// implementation for `LowerHex` and `UpperHex`
#[derive(Debug)]
pub struct BinaryRef<'a>(pub &'a [u8]);

impl<'a> From<&'a Binary> for BinaryRef<'a> {
    fn from(value: &'a Binary) -> Self {
        Self(value.as_ref())
    }
}

impl<'a> Serialize for BinaryRef<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(self.0)
    }
}

impl<'de> de::Deserialize<'de> for BinaryRef<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        serde::Deserialize::deserialize(deserializer).map(BinaryRef)
    }
}

impl<'a> LowerHex for BinaryRef<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(f, "{:x}", byte)?;
        }
        Ok(())
    }
}

impl<'a> UpperHex for BinaryRef<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(f, "{:X}", byte)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{from_slice, primitives::Binary, to_vec};

    use super::BinaryRef;

    #[test]
    fn test_format_lower_hex() {
        let bref = BinaryRef(b"amqp");
        let s = format!("{:x}", bref);
        assert_eq!("616d7170", s);
    }

    #[test]
    fn test_format_upper_hex() {
        let bref = BinaryRef(b"amqp");
        let s = format!("{:X}", bref);
        assert_eq!("616D7170", s);
    }

    #[test]
    fn test_serialize_binary_ref() {
        let bref = BinaryRef(b"amqp");
        let buf = to_vec(&bref).unwrap();

        let b = Binary::from("amqp");
        let expected = to_vec(&b).unwrap();

        assert_eq!(buf, expected);
    }

    #[test]
    fn test_deserialize_binary_ref() {
        let expected = Binary::from("amqp");
        let buf = to_vec(&expected).unwrap();

        let bref: BinaryRef = from_slice(&buf).unwrap();
        assert_eq!(bref.0, expected.as_ref());
    }
}
