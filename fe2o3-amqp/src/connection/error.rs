use std::io;

use fe2o3_amqp_types::{
    definitions::{AmqpError, ConnectionError},
    primitives::Binary,
    sasl::SaslCode,
};
use tokio::{sync::mpsc, task::JoinError};

use crate::transport::{self, error::NegotiationError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO Error {0:?}")]
    Io(#[from] io::Error),

    #[error(transparent)]
    JoinError(JoinError),

    #[error("Exceeding channel-max")]
    ChannelMaxReached,

    #[error("AMQP error {:?}, {:?}", .condition, .description)]
    AmqpError {
        condition: AmqpError,
        description: Option<String>,
    },

    #[error("Connection error {:?}, {:?}", .condition, .description)]
    ConnectionError {
        condition: ConnectionError,
        description: Option<String>,
    },
}

impl<T> From<mpsc::error::SendError<T>> for Error
where
    T: std::fmt::Debug,
{
    fn from(err: mpsc::error::SendError<T>) -> Self {
        Self::Io(io::Error::new(io::ErrorKind::Other, err.to_string()))
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

impl Error {
    pub fn amqp_error(
        condition: impl Into<AmqpError>,
        description: impl Into<Option<String>>,
    ) -> Self {
        Self::AmqpError {
            condition: condition.into(),
            description: description.into(),
        }
    }

    pub fn connection_error(
        condition: impl Into<ConnectionError>,
        description: impl Into<Option<String>>,
    ) -> Self {
        Self::ConnectionError {
            condition: condition.into(),
            description: description.into(),
        }
    }
}

impl From<transport::Error> for Error {
    fn from(err: transport::Error) -> Self {
        match err {
            transport::Error::Io(e) => Self::Io(e),
            transport::Error::IdleTimeout => Self::ConnectionError {
                condition: ConnectionError::ConnectionForced,
                description: Some("Idle timeout".to_string()),
            },
            transport::Error::AmqpError {
                condition,
                description,
            } => Self::AmqpError {
                condition,
                description,
            },
            transport::Error::FramingError => Self::ConnectionError {
                condition: ConnectionError::FramingError,
                description: None,
            },
        }
    }
}

/// Error associated with allocation of new session
#[derive(Debug, thiserror::Error)]
pub enum AllocSessionError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("Illegal local state")]
    IllegalState,

    #[error("Reached connection channel max")]
    ChannelMaxReached,
}

impl<T> From<mpsc::error::SendError<T>> for AllocSessionError
where
    T: std::fmt::Debug,
{
    fn from(err: mpsc::error::SendError<T>) -> Self {
        Self::Io(io::Error::new(io::ErrorKind::Other, err.to_string()))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    #[error("IO Error {0:?}")]
    Io(#[from] io::Error),

    #[error(transparent)]
    UrlError(#[from] url::ParseError),

    #[error("Protocol header mismatch {0:?}")]
    ProtocolHeaderMismatch([u8; 8]),

    #[error("Invalid domain")]
    InvalidDomain,

    #[error("TLS client config is not found")]
    TlsClientConfigNotFound,

    #[error(r#"Invalid scheme. Only "amqp" and "amqps" are supported."#)]
    InvalidScheme,

    #[error("AMQP error {:?}, {:?}", .condition, .description)]
    AmqpError {
        condition: AmqpError,
        description: Option<String>,
    },

    #[error("Connection error {:?}, {:?}", .condition, .description)]
    ConnectionError {
        condition: ConnectionError,
        description: Option<String>,
    },

    #[error("SASL error code {:?}, additional data: {:?}", .code, .additional_data)]
    SaslError {
        code: SaslCode,
        additional_data: Option<Binary>,
    },
}

impl From<NegotiationError> for OpenError {
    fn from(err: NegotiationError) -> Self {
        match err {
            NegotiationError::Io(err) => Self::Io(err),
            NegotiationError::ProtocolHeaderMismatch(buf) => Self::ProtocolHeaderMismatch(buf),
            NegotiationError::InvalidDomain => Self::InvalidDomain,
            NegotiationError::AmqpError {
                condition,
                description,
            } => Self::AmqpError {
                condition,
                description,
            },
            NegotiationError::SaslError {
                code,
                additional_data,
            } => Self::SaslError {
                code,
                additional_data,
            },
        }
    }
}

impl From<AmqpError> for OpenError {
    fn from(err: AmqpError) -> Self {
        Self::AmqpError {
            condition: err,
            description: None,
        }
    }
}
