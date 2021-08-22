//! Custom structs that hold bytes for decimal types

pub const DECIMAL32: &str = "DECIMAL32";
pub const DECIMAL64: &str = "DECIMAL64";
pub const DECIMAL128: &str = "DECIMAL128";

/// TODO: implement Serialize and Deserialize
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Dec32([u8; 4]);

/// TODO: implement Serialize and Deserialize
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Dec64([u8; 8]);

/// TODO: implement Serialize and Deserialize
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Dec128([u8; 16]);
