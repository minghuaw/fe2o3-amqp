use std::io;

use bytes::Bytes;
use fe2o3_amqp_types::{definitions::AmqpError, primitives::Binary, sasl::SaslCode};

use crate::{frames, sasl_profile};

/// Transport error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// IO error
    #[error("IO Error {0:?}")]
    Io(#[from] io::Error),

    /// Idle timeout
    #[error("Idle timeout")]
    IdleTimeout,

    /// AMQP error
    #[error("AMQP error {:?}, {:?}", .condition, .description)]
    AmqpError {
        /// Error condition
        condition: AmqpError,

        /// Error description
        description: Option<String>,
    },

    /// Connection error: framing error
    #[error("Connection error: framing error")]
    FramingError,
}

// impl Error {
//     pub(crate) fn amqp_error(
//         condition: impl Into<AmqpError>,
//         description: impl Into<Option<String>>,
//     ) -> Self {
//         Self::AmqpError {
//             condition: condition.into(),
//             description: description.into(),
//         }
//     }
// }

// TODO: What about encode error?
impl From<serde_amqp::Error> for Error {
    fn from(err: serde_amqp::Error) -> Self {
        match err {
            serde_amqp::Error::Io(e) => Self::Io(e),
            e => {
                let description = e.to_string();
                Self::AmqpError {
                    condition: AmqpError::DecodeError,
                    description: Some(description),
                }
            }
        }
    }
}

impl From<AmqpError> for Error {
    fn from(err: AmqpError) -> Self {
        Self::AmqpError {
            condition: err,
            description: None,
        }
    }
}

// impl From<ConnectionError> for Error {
//     fn from(err: ConnectionError) -> Self {
//         Self::ConnectionError {
//             condition: err,
//             description: None,
//         }
//     }
// }

impl From<frames::Error> for Error {
    fn from(err: frames::Error) -> Self {
        match err {
            frames::Error::Io(io) => Self::Io(io),
            frames::Error::DecodeError => Self::AmqpError {
                condition: AmqpError::DecodeError,
                description: None,
            },
            frames::Error::NotImplemented => Self::AmqpError {
                condition: AmqpError::NotImplemented,
                description: None,
            },
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

    // #[error("AMQP error {:?}, {:?}", .condition, .description)]
    // AmqpError {
    //     condition: AmqpError,
    //     description: Option<String>,
    // },
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

// impl From<AmqpError> for NegotiationError {
//     fn from(err: AmqpError) -> Self {
//         Self::AmqpError {
//             condition: err,
//             description: None,
//         }
//     }
// }
