//! Implements errors associated with the connection

use std::io;

use fe2o3_amqp_types::{
    definitions::{self, AmqpError, ConnectionError, ErrorCondition},
    primitives::Binary,
    sasl::SaslCode,
};
use tokio::{sync::mpsc, task::JoinError};

use crate::transport::{self, error::NegotiationError};

/// Errors associated with [`crate::Connection`]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// IO error
    #[error("IO Error {0:?}")]
    Io(#[from] io::Error),

    /// This could occur only when the user attempts to close the connection
    #[error(transparent)]
    JoinError(JoinError),

    /// A local error
    #[error("Local error {:?}", .0)]
    Local(definitions::Error),

    /// The remote peer closed with the provided error
    #[error("Remote error {:?}", .0)]
    Remote(definitions::Error),
}

impl<T> From<mpsc::error::SendError<T>> for Error
where
    T: std::fmt::Debug,
{
    fn from(err: mpsc::error::SendError<T>) -> Self {
        Self::Io(io::Error::new(io::ErrorKind::Other, err.to_string()))
    }
}

impl Error {
    pub(crate) fn amqp_error(
        condition: impl Into<AmqpError>,
        description: impl Into<Option<String>>,
    ) -> Self {
        Self::Local(definitions::Error {
            condition: ErrorCondition::AmqpError(condition.into()),
            description: description.into(),
            info: None,
        })
    }

    pub(crate) fn connection_error(
        condition: impl Into<ConnectionError>,
        description: impl Into<Option<String>>,
    ) -> Self {
        Self::Local(definitions::Error {
            condition: ErrorCondition::ConnectionError(condition.into()),
            description: description.into(),
            info: None,
        })
    }
}

impl From<transport::Error> for Error {
    fn from(err: transport::Error) -> Self {
        match err {
            transport::Error::Io(e) => Self::Io(e),
            transport::Error::IdleTimeout => Self::connection_error(
                ConnectionError::ConnectionForced,
                Some("Idle timeout".to_string()),
            ),
            transport::Error::AmqpError {
                condition,
                description,
            } => Self::amqp_error(condition, description),
            transport::Error::FramingError => {
                Self::connection_error(ConnectionError::FramingError, None)
            }
        }
    }
}

/// Error associated with allocation of new session
#[derive(Debug, thiserror::Error)]
pub(crate) enum AllocSessionError {
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

/// Error associated with openning a connection
#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    /// IO error
    #[error("IO Error {0:?}")]
    Io(#[from] io::Error),

    /// Error parsing the url
    #[error(transparent)]
    UrlError(#[from] url::ParseError),

    /// Domain is invalid or not found
    #[error("Invalid domain")]
    InvalidDomain,

    /// Missing client config for TLS connection
    #[error("TLS client config is not found")]
    TlsClientConfigNotFound,

    /// Scheme is invalid or not found
    #[error(r#"Invalid scheme. Only "amqp" and "amqps" are supported."#)]
    InvalidScheme,

    /// Protocol negotiation failed due to protocol header mismatch
    #[error("Protocol header mismatch. Found {0:?}")]
    ProtocolHeaderMismatch([u8; 8]),

    /// SASL negotiation failed
    #[error("SASL error code {:?}, additional data: {:?}", .code, .additional_data)]
    SaslError {
        /// SASL outcome code
        code: SaslCode,
        /// Additional information for the failed negotiation
        additional_data: Option<Binary>,
    },

    /// A local error
    #[error("Local error {:?}", .0)]
    LocalError(definitions::Error),

    /// The remote peer closed the connection with the provided error
    #[error("Remote error {:?}", .0)]
    RemoteError(definitions::Error),
}

impl From<NegotiationError> for OpenError {
    fn from(err: NegotiationError) -> Self {
        match err {
            NegotiationError::Io(err) => Self::Io(err),
            NegotiationError::ProtocolHeaderMismatch(buf) => Self::ProtocolHeaderMismatch(buf),
            NegotiationError::InvalidDomain => Self::InvalidDomain,
            NegotiationError::SaslError {
                code,
                additional_data,
            } => Self::SaslError {
                code,
                additional_data,
            },
            NegotiationError::DecodeError => {
                Self::LocalError(definitions::Error::new(AmqpError::DecodeError, None, None))
            }
            NegotiationError::NotImplemented(description) => Self::LocalError(
                definitions::Error::new(AmqpError::NotImplemented, description, None),
            ),
            NegotiationError::IllegalState => {
                Self::LocalError(definitions::Error::new(AmqpError::IllegalState, None, None))
            }
        }
    }
}
