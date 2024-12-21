//! Custom structs that hold bytes for decimal types

use std::convert::TryFrom;

use serde::de;
use serde::ser;
use serde_bytes::Bytes;

use crate::error::Error;

mod dec32 {
    // use serde_bytes::ByteBuf;

    use crate::{__constants::DECIMAL32, fixed_width::DECIMAL32_WIDTH};

    use super::*;

    /// 32-bit decimal number (IEEE 754-2008 decimal32).
    ///
    /// encoding name = "ieee-754", encoding code = 0x74
    /// category = fixed, width = 4
    /// label = "IEEE 754-2008 decimal32 using the Binary Integer Decimal encoding"
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Dec32([u8; DECIMAL32_WIDTH]);

    impl Dec32 {
        /// Consume the wrapper into the inner bytes
        pub fn into_inner(self) -> [u8; DECIMAL32_WIDTH] {
            self.0
        }
    }

    impl From<[u8; DECIMAL32_WIDTH]> for Dec32 {
        fn from(val: [u8; DECIMAL32_WIDTH]) -> Self {
            Self(val)
        }
    }

    impl From<Dec32> for [u8; DECIMAL32_WIDTH] {
        fn from(val: Dec32) -> Self {
            val.0
        }
    }

    impl TryFrom<&[u8]> for Dec32 {
        type Error = Error;

        fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
            if value.len() != DECIMAL32_WIDTH {
                return Err(Error::InvalidLength);
            }

            let mut buf = [0u8; DECIMAL32_WIDTH];
            buf.copy_from_slice(&value[..DECIMAL32_WIDTH]);
            Ok(Self(buf))
        }
    }

    impl ser::Serialize for Dec32 {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.serialize_newtype_struct(DECIMAL32, Bytes::new(&self.0))
        }
    }

    struct Visitor {}

    impl de::Visitor<'_> for Visitor {
        type Value = Dec32;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("struct Dec32")
        }

        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Dec32::try_from(v).map_err(|err| de::Error::custom(err.to_string()))
        }
    }

    impl<'de> de::Deserialize<'de> for Dec32 {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_newtype_struct(DECIMAL32, Visitor {})
        }
    }
}

mod dec64 {
    // use serde_bytes::ByteBuf;

    use crate::{__constants::DECIMAL64, fixed_width::DECIMAL64_WIDTH};

    use super::*;

    /// 64-bit decimal number (IEEE 754-2008 decimal64).
    ///
    /// encoding name = "ieee-754", encoding code = 0x84
    /// category = fixed, width = 8
    /// label = "IEEE 754-2008 decimal64 using the Binary Integer Decimal encoding"
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Dec64([u8; DECIMAL64_WIDTH]);

    impl Dec64 {
        /// Consumes the wrapper into the inner bytes
        pub fn into_inner(self) -> [u8; DECIMAL64_WIDTH] {
            self.0
        }
    }

    impl From<[u8; DECIMAL64_WIDTH]> for Dec64 {
        fn from(val: [u8; DECIMAL64_WIDTH]) -> Self {
            Self(val)
        }
    }

    impl TryFrom<&[u8]> for Dec64 {
        type Error = Error;

        fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
            if value.len() != DECIMAL64_WIDTH {
                return Err(Error::InvalidLength);
            }

            let mut buf = [0u8; DECIMAL64_WIDTH];
            buf.copy_from_slice(&value[..DECIMAL64_WIDTH]);
            Ok(Self(buf))
        }
    }

    impl ser::Serialize for Dec64 {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.serialize_newtype_struct(DECIMAL64, Bytes::new(&self.0))
        }
    }

    struct Visitor {}

    impl de::Visitor<'_> for Visitor {
        type Value = Dec64;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("struct Dec64")
        }

        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Dec64::try_from(v).map_err(|err| de::Error::custom(err.to_string()))
        }
    }

    impl<'de> de::Deserialize<'de> for Dec64 {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_newtype_struct(DECIMAL64, Visitor {})
        }
    }
}

mod dec128 {
    // use serde_bytes::ByteBuf;

    use crate::{__constants::DECIMAL128, fixed_width::DECIMAL128_WIDTH};

    use super::*;

    /// 128-bit decimal number (IEEE 754-2008 decimal128).
    ///
    /// encoding name = "ieee-754", encoding code = 0x94
    /// category = fixed, width = 16
    /// label = "IEEE 754-2008 decimal128 using the Binary Integer Decimal encoding"
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Dec128([u8; DECIMAL128_WIDTH]);

    impl Dec128 {
        /// Consumes the wrapper into the inner bytes
        pub fn into_inner(self) -> [u8; DECIMAL128_WIDTH] {
            self.0
        }
    }

    impl From<[u8; DECIMAL128_WIDTH]> for Dec128 {
        fn from(val: [u8; DECIMAL128_WIDTH]) -> Self {
            Self(val)
        }
    }

    impl TryFrom<&[u8]> for Dec128 {
        type Error = Error;

        fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
            if value.len() != DECIMAL128_WIDTH {
                return Err(Error::InvalidLength);
            }

            let mut buf = [0u8; DECIMAL128_WIDTH];
            buf.copy_from_slice(&value[..DECIMAL128_WIDTH]);
            Ok(Self(buf))
        }
    }

    impl ser::Serialize for Dec128 {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.serialize_newtype_struct(DECIMAL128, Bytes::new(&self.0))
        }
    }

    struct Visitor {}

    impl de::Visitor<'_> for Visitor {
        type Value = Dec128;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("struct Dec128")
        }

        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Dec128::try_from(v).map_err(|err| de::Error::custom(err.to_string()))
        }
    }

    impl<'de> de::Deserialize<'de> for Dec128 {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_newtype_struct(DECIMAL128, Visitor {})
        }
    }
}

pub use dec128::*;
pub use dec32::*;
pub use dec64::*;
