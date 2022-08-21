//! Implements errors associated with the connection

use std::io;

use bytes::Bytes;
use fe2o3_amqp_types::{definitions, primitives::Binary, sasl::SaslCode};
use tokio::{sync::mpsc, task::JoinError};

use crate::transport::{self, error::NegotiationError};

#[cfg(feature = "scram")]
use crate::sasl_profile::ScramErrorKind;

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
    #[error("TLS connector is not found")]
    TlsConnectorNotFound,

    /// Scheme is invalid or not found
    #[error(r#"Invalid scheme. Only "amqp" and "amqps" are supported."#)]
    InvalidScheme,

    /// Protocol negotiation failed due to protocol header mismatch
    #[error("Protocol header mismatch. Found {0:?}")]
    ProtocolHeaderMismatch(Bytes),

    /// SASL negotiation failed
    #[error("SASL error code {:?}, additional data: {:?}", .code, .additional_data)]
    SaslError {
        /// SASL outcome code
        code: SaslCode,
        /// Additional information for the failed negotiation
        additional_data: Option<Binary>,
    },

    /// Error with SCRAM
    #[cfg_attr(docsrs, doc(cfg(feature = "scram")))]
    #[cfg(feature = "scram")]
    #[error(transparent)]
    ScramError(#[from] ScramErrorKind),

    /// Illegal local connection state
    #[error("Illegal local state")]
    IllegalState,

    /// Not implemented
    #[error("Not implemented")]
    NotImplemented(Option<String>),

    /// Decode error
    #[error("Decode error")]
    DecodeError(String),

    /// Transport error
    #[error(transparent)]
    TransportError(#[from] transport::Error),

    /// Remote peer closed connection during openning process
    #[error("Remote peer closed")]
    RemoteClosed,

    /// Remote peer closed connection with error during openning process
    #[error("Remote peer closed connection with error {}", .0)]
    RemoteClosedWithError(definitions::Error),
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
            NegotiationError::DecodeError(val) => Self::DecodeError(val),
            NegotiationError::NotImplemented(description) => Self::NotImplemented(description),
            NegotiationError::IllegalState => Self::IllegalState,
            
            #[cfg(feature = "scram")]
            NegotiationError::ScramError(e) => Self::ScramError(e),
        }
    }
}

/// Error the connection state
#[derive(Debug, thiserror::Error)]
pub(crate) enum ConnectionStateError {
    /// Illegal local connection state
    #[error("Illegal local state")]
    IllegalState,

    /// Remote peer closed connection
    #[error("Remote peer closed")]
    RemoteClosed,

    /// Remote peer closed connection with error
    #[error("Remote peer closed connection with error {}", .0)]
    RemoteClosedWithError(definitions::Error),

    /// Transport error
    #[error(transparent)]
    TransportError(#[from] transport::Error),
}

pub(crate) type CloseError = ConnectionStateError;

impl From<ConnectionStateError> for OpenError {
    fn from(error: ConnectionStateError) -> Self {
        match error {
            ConnectionStateError::IllegalState => Self::IllegalState,
            ConnectionStateError::RemoteClosed => Self::RemoteClosed,
            ConnectionStateError::RemoteClosedWithError(val) => Self::RemoteClosedWithError(val),
            ConnectionStateError::TransportError(val) => Self::TransportError(val),
        }
    }
}

/// Error with connection
#[derive(Debug, thiserror::Error)]
pub(crate) enum ConnectionInnerError {
    /// Transport error
    #[error(transparent)]
    TransportError(#[from] transport::Error),

    /// Illegal local connection state
    #[error("Illegal local state")]
    IllegalState,

    /// Not implemented
    #[error("Not implemented {:?}", .0)]
    NotImplemented(Option<String>),

    /// Not found
    #[error("Not found {:?}", .0)]
    NotFound(Option<String>),

    /// Remote peer closed connection
    #[error("Remote peer closed")]
    RemoteClosed,

    /// Remote peer closed connection with error
    #[error("Remote peer closed connection with error {}", .0)]
    RemoteClosedWithError(definitions::Error),
}

impl<T> From<mpsc::error::SendError<T>> for ConnectionInnerError
where
    T: std::fmt::Debug,
{
    fn from(_: mpsc::error::SendError<T>) -> Self {
        Self::NotFound(Some("Session is not found".to_string()))
    }
}

impl From<ConnectionStateError> for ConnectionInnerError {
    fn from(error: ConnectionStateError) -> Self {
        match error {
            ConnectionStateError::IllegalState => Self::IllegalState,
            ConnectionStateError::RemoteClosed => Self::RemoteClosed,
            ConnectionStateError::RemoteClosedWithError(val) => Self::RemoteClosedWithError(val),
            ConnectionStateError::TransportError(val) => Self::TransportError(val),
        }
    }
}

/// Error with connection
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Transport error
    #[error(transparent)]
    TransportError(#[from] transport::Error),

    /// Illegal local connection state
    #[error("Illegal local state")]
    IllegalState,

    /// Not implemented
    #[error("Not implemented {:?}", .0)]
    NotImplemented(Option<String>),

    /// Session is not found
    #[error("Not found {:?}", .0)]
    NotFound(Option<String>),

    /// Not allowed
    #[error("Not allowd {:?}", .0)]
    NotAllowed(Option<String>),

    /// Remote peer closed connection
    #[error("Remote peer closed")]
    RemoteClosed,

    /// Remote peer closed connection with error
    #[error("Remote peer closed connection with error {}", .0)]
    RemoteClosedWithError(definitions::Error),

    /// This could occur only when the user attempts to close the connection
    #[error(transparent)]
    JoinError(#[from] JoinError),
}

impl From<ConnectionInnerError> for Error {
    fn from(error: ConnectionInnerError) -> Self {
        match error {
            ConnectionInnerError::TransportError(val) => Self::TransportError(val),
            ConnectionInnerError::IllegalState => Self::IllegalState,
            ConnectionInnerError::NotImplemented(val) => Self::NotImplemented(val),
            ConnectionInnerError::NotFound(val) => Self::NotFound(val),
            ConnectionInnerError::RemoteClosed => Self::RemoteClosed,
            ConnectionInnerError::RemoteClosedWithError(val) => Self::RemoteClosedWithError(val),
        }
    }
}

impl From<ConnectionStateError> for Error {
    fn from(error: ConnectionStateError) -> Self {
        match error {
            ConnectionStateError::IllegalState => Self::IllegalState,
            ConnectionStateError::RemoteClosed => Self::RemoteClosed,
            ConnectionStateError::RemoteClosedWithError(val) => Self::RemoteClosedWithError(val),
            ConnectionStateError::TransportError(val) => Self::TransportError(val),
        }
    }
}

/// Error associated with allocation of new session
#[derive(Debug, thiserror::Error)]
pub(crate) enum AllocSessionError {
    #[error("Illegal local state")]
    IllegalState,

    #[error("Reached connection channel max")]
    ChannelMaxReached,
}

pub(crate) enum DeallcoSessionError {
    IllegalState,
}
