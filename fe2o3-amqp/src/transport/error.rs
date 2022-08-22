use std::io;

use bytes::Bytes;
use fe2o3_amqp_types::{primitives::Binary, sasl::SaslCode};

use crate::{frames, sasl_profile, auth::error::ScramErrorKind};

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
    DecodeError(String),

    /// Not implemented
    #[error("Not implemented")]
    NotImplemented(Option<String>),

    /// Connection error: framing error
    #[error("Connection error: framing error")]
    FramingError,
}

impl From<serde_amqp::Error> for Error {
    fn from(err: serde_amqp::Error) -> Self {
        match err {
            serde_amqp::Error::Io(e) => Self::Io(e),
            other => Self::DecodeError(other.to_string()),
        }
    }
}

impl From<frames::Error> for Error {
    fn from(err: frames::Error) -> Self {
        match err {
            frames::Error::Io(io) => Self::Io(io),
            frames::Error::DecodeError(val) => Self::DecodeError(val),
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
    DecodeError(String),

    #[error("Not implemented")]
    NotImplemented(Option<String>),

    #[error("Illegal state")]
    IllegalState,

    #[error("SASL error code {:?}, additional data: {:?}", .code, .additional_data)]
    SaslError {
        code: SaslCode,
        additional_data: Option<Binary>,
    },

    /// Error with SCRAM
    #[cfg_attr(docsrs, doc(cfg(feature = "scram")))]
    #[cfg(feature = "scram")]
    #[error(transparent)]
    ScramError(#[from] ScramErrorKind),
}

// TODO: What about encode error?
impl From<frames::Error> for NegotiationError {
    fn from(err: frames::Error) -> Self {
        match err {
            frames::Error::Io(err) => Self::Io(err),
            frames::Error::DecodeError(val) => Self::DecodeError(val),
            frames::Error::NotImplemented => Self::NotImplemented(None),
        }
    }
}

impl From<sasl_profile::Error> for NegotiationError {
    fn from(err: sasl_profile::Error) -> Self {
        match err {
            sasl_profile::Error::NotImplemented(msg) => Self::NotImplemented(msg),

            #[cfg(feature = "scram")]
            sasl_profile::Error::ScramError(scram_error) => Self::ScramError(scram_error),
        }
    }
}
