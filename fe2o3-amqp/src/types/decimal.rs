//! Custom structs that hold bytes for decimal types

use std::convert::TryFrom;

use serde::ser;
use serde_bytes::Bytes;

use crate::error::Error;

pub const DECIMAL32: &str = "DECIMAL32";
pub const DECIMAL64: &str = "DECIMAL64";
pub const DECIMAL128: &str = "DECIMAL128";

/// TODO: implement Serialize and Deserialize
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Dec32([u8; 4]);

impl From<[u8; 4]> for Dec32 {
    fn from(val: [u8; 4]) -> Self {
        Self(val)
    }
}

impl TryFrom<&[u8]> for Dec32 {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != 4 {
            return Err(Error::InvalidLength)
        }

        let mut buf = [0u8; 4];
        buf.copy_from_slice(&value[..4]);
        Ok(Self(buf))
    }
}

impl ser::Serialize for Dec32 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer 
    {
        serializer.serialize_newtype_struct(DECIMAL32, Bytes::new(&self.0))
    }
}

/// TODO: implement Serialize and Deserialize
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Dec64([u8; 8]);

impl From<[u8; 8]> for Dec64 {
    fn from(val: [u8; 8]) -> Self {
        Self(val)
    }
}

impl TryFrom<&[u8]> for Dec64 {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != 8 {
            return Err(Error::InvalidLength)
        }

        let mut buf = [0u8; 8];
        buf.copy_from_slice(&value[..8]);
        Ok(Self(buf))
    }
}

impl ser::Serialize for Dec64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer 
    {
        serializer.serialize_newtype_struct(DECIMAL64, Bytes::new(&self.0))
    }
}

/// TODO: implement Serialize and Deserialize
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Dec128([u8; 16]);

impl From<[u8; 16]> for Dec128 {
    fn from(val: [u8; 16]) -> Self {
        Self(val)
    }
}

impl TryFrom<&[u8]> for Dec128 {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != 16 {
            return Err(Error::InvalidLength)
        }

        let mut buf = [0u8; 16];
        buf.copy_from_slice(&value[..16]);
        Ok(Self(buf))
    }
}

impl ser::Serialize for Dec128 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer 
    {
        serializer.serialize_newtype_struct(DECIMAL128, Bytes::new(&self.0))
    }
}
