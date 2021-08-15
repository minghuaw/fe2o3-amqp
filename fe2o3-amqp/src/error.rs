use serde::{de, ser};
use std::fmt::Display;

// pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Message {0}")]
    Message(String),

    #[error("IO {0}")]
    Io(std::io::Error),

    #[error("Invalid constructor")]
    InvalidConstructor,

    #[error("EOF while parsing")]
    EofWhileParsing
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
