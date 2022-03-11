//! Custom error

use serde::{de, ser};
use std::fmt::Display;

// pub type Result<T> = core::result::Result<T, Error>;

/// Custom serialization/deserialization errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Custom error with message
    #[error("Message {0}")]
    Message(String),

    /// IO error
    #[error("IO {0}")]
    Io(std::io::Error),

    /// Invalid format code
    #[error("Invalid format code")]
    InvalidFormatCode,

    /// Invalid value
    #[error("Invalid value")]
    InvalidValue,

    /// A described type is found while a primitive type is expected
    #[error("Expecting non-described constructor")]
    IsDescribedType,

    /// Found invalid UTF-8 encoding
    #[error("Invalid UTF-8 encoding")]
    InvalidUtf8Encoding,

    /// Sequence type length mismatch
    #[error("Sequence length mismatch")]
    SequenceLengthMismatch,

    /// Length is invalid
    #[error("Invalid length")]
    InvalidLength,
}

impl Error {
    pub(crate) fn too_long() -> Self {
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "Too long");
        Self::Io(io_err)
    }
}

impl ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self::Message(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Self::Message(msg.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(_: std::string::FromUtf8Error) -> Self {
        Error::InvalidUtf8Encoding
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(_: std::str::Utf8Error) -> Self {
        Error::InvalidUtf8Encoding
    }
}
