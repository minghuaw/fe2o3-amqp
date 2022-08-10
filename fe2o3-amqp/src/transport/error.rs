use std::io;

use bytes::Bytes;
use fe2o3_amqp_types::{primitives::Binary, sasl::SaslCode};

use crate::{frames, sasl_profile};

/// Transport error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// IO error
    #[error("IO Error {0:?}")]
    Io(#[from] io::Error),

    /// Idle timeout
    #[error("Idle timeout")]
    IdleTimeoutElapsed,

    /// Decode error
    #[error("Decode error")]
    DecodeError,

    /// Not implemented
    #[error("Not implemented")]
    NotImplemented(Option<String>),

    /// Connection error: framing error
    #[error("Connection error: framing error")]
    FramingError,
}

// TODO: What about encode error?
impl From<serde_amqp::Error> for Error {
    fn from(err: serde_amqp::Error) -> Self {
        match err {
            serde_amqp::Error::Io(e) => Self::Io(e),
            _e => Self::DecodeError,
        }
    }
}

impl From<frames::Error> for Error {
    fn from(err: frames::Error) -> Self {
        match err {
            frames::Error::Io(io) => Self::Io(io),
            frames::Error::DecodeError => Self::DecodeError,
            frames::Error::NotImplemented => Self::NotImplemented(None),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NegotiationError {
    #[error("IO Error {0:?}")]
    Io(#[from] io::Error),

    #[error("Protocol header mismatch {0:?}")]
    ProtocolHeaderMismatch(Bytes),

    #[error("Invalid domain")]
    InvalidDomain,

    #[error("Decode error")]
    DecodeError,

    #[error("Not implemented")]
    NotImplemented(Option<String>),

    #[error("Illegal state")]
    IllegalState,

    #[error("SASL error code {:?}, additional data: {:?}", .code, .additional_data)]
    SaslError {
        code: SaslCode,
        additional_data: Option<Binary>,
    },
}

// TODO: What about encode error?
impl From<frames::Error> for NegotiationError {
    fn from(err: frames::Error) -> Self {
        match err {
            frames::Error::Io(err) => Self::Io(err),
            frames::Error::DecodeError => Self::DecodeError,
            frames::Error::NotImplemented => Self::NotImplemented(None),
        }
    }
}

impl From<sasl_profile::Error> for NegotiationError {
    fn from(err: sasl_profile::Error) -> Self {
        match err {
            sasl_profile::Error::NotImplemented(msg) => Self::NotImplemented(msg),
        }
    }
}
