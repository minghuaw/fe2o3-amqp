//! Implements errors associated with the connection

use std::{convert::Infallible, io};

use bytes::Bytes;
use fe2o3_amqp_types::{definitions, primitives::Binary, sasl::SaslCode};
use tokio::sync::mpsc;

use crate::{
    connection::ConnectionOutcome,
    transport::{self, error::NegotiationError},
};

cfg_scram! {
    use crate::auth::error::ScramErrorKind;
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
    #[error("TLS connector is not found")]
    TlsConnectorNotFound,

    /// Scheme is invalid or not found
    #[error(r#"Invalid scheme. Only "amqp" and "amqps" are supported."#)]
    InvalidScheme,

    /// Scheme is invalid for SaslProfile::External
    #[error(r#"SASL EXTERNAL mechanism requires "amqps" scheme."#)]
    InvalidSchemeForSaslExternal,

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

    /// The peer sent a frame that is not permitted in the current connection state
    #[error("The peer sent a frame that is not permitted in the current connection state")]
    IllegalState,

    /// The transport closed before the connection was opened
    #[error("The connection was lost")]
    ConnectionLost,

    /// An internal invariant was violated (defensive)
    ///
    /// This is a safeguard for a path that is impossible by construction; it
    /// cannot occur unless the library breaks its own invariants.
    #[error("An internal invariant was violated")]
    InvariantViolation,

    /// Not implemented
    #[error("Not implemented {:?}", .0)]
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
            NegotiationError::InvariantViolation => Self::InvariantViolation,

            #[cfg(feature = "scram")]
            NegotiationError::ScramError(e) => Self::ScramError(e),
        }
    }
}

impl From<Infallible> for OpenError {
    fn from(_: Infallible) -> Self {
        unreachable!("Infallible cannot be constructed")
    }
}

/// Error the connection state
#[derive(Debug, thiserror::Error)]
pub(crate) enum ConnectionStateError {
    /// The peer sent a frame that is not permitted in the current connection state
    #[error("The peer sent a frame that is not permitted in the current connection state")]
    IllegalState,

    /// An internal invariant was violated (defensive)
    ///
    /// This is a safeguard for a path that is impossible by construction; it
    /// cannot occur unless the library breaks its own invariants.
    #[error("An internal invariant was violated")]
    InvariantViolation,

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
            ConnectionStateError::InvariantViolation => Self::InvariantViolation,
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

    /// The peer sent a frame that is not permitted in the current connection state
    #[error("The peer sent a frame that is not permitted in the current connection state")]
    IllegalState,

    /// The transport closed without the AMQP close exchange
    #[error("The connection was lost")]
    ConnectionLost,

    /// An internal invariant was violated (defensive)
    ///
    /// This is a safeguard for a path that is impossible by construction; it
    /// cannot occur unless the library breaks its own invariants.
    #[error("An internal invariant was violated")]
    InvariantViolation,

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
            ConnectionStateError::InvariantViolation => Self::InvariantViolation,
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

    /// The peer sent a frame that is not permitted in the current connection state
    #[error("The peer sent a frame that is not permitted in the current connection state")]
    IllegalState,

    /// The transport closed without the AMQP close exchange
    #[error("The connection was lost")]
    ConnectionLost,

    /// An internal invariant was violated (defensive)
    ///
    /// This is a safeguard for a path that is impossible by construction; it
    /// cannot occur unless the library breaks its own invariants.
    #[error("An internal invariant was violated")]
    InvariantViolation,

    /// An internal failure that can occur in principle
    ///
    /// Reported when the connection stopped because an internal operation
    /// failed without a more specific classification, e.g. the engine task
    /// stopped without reporting its outcome. Both this and
    /// [`Self::InvariantViolation`] answer `amqp:internal-error`.
    #[error("An internal error occurred")]
    InternalError,

    /// Not implemented
    #[error("Not implemented {:?}", .0)]
    NotImplemented(Option<String>),

    /// Session is not found
    #[error("Not found {:?}", .0)]
    NotFound(Option<String>),

    /// The connection is already closed and its outcome was already observed
    #[error("The connection is already closed")]
    AlreadyClosed,
}

impl From<ConnectionInnerError> for Error {
    fn from(error: ConnectionInnerError) -> Self {
        match error {
            ConnectionInnerError::TransportError(val) => Self::TransportError(val),
            ConnectionInnerError::IllegalState => Self::IllegalState,
            ConnectionInnerError::ConnectionLost => Self::ConnectionLost,
            ConnectionInnerError::InvariantViolation => Self::InvariantViolation,
            ConnectionInnerError::NotImplemented(val) => Self::NotImplemented(val),
            ConnectionInnerError::NotFound(val) => Self::NotFound(val),
            // A remote close is converted into the connection outcome by the
            // engine before it reaches this conversion (see `event_loop`).
            ConnectionInnerError::RemoteClosed | ConnectionInnerError::RemoteClosedWithError(_) => {
                Self::InternalError
            }
        }
    }
}

impl From<ConnectionStateError> for Error {
    fn from(error: ConnectionStateError) -> Self {
        match error {
            ConnectionStateError::IllegalState => Self::IllegalState,
            ConnectionStateError::InvariantViolation => Self::InvariantViolation,
            // A remote close is converted into the connection outcome by the
            // engine before it reaches this conversion (see `event_loop`).
            ConnectionStateError::RemoteClosed | ConnectionStateError::RemoteClosedWithError(_) => {
                Self::InternalError
            }
            ConnectionStateError::TransportError(val) => Self::TransportError(val),
        }
    }
}

/// Error associated with allocation of new session
#[derive(Debug, thiserror::Error)]
pub(crate) enum AllocSessionError {
    /// The connection has not been opened yet
    #[error("The connection has not been opened")]
    ConnectionNotOpened,

    /// The connection stopped before the session was allocated
    #[error("The connection stopped: {:?}", .0)]
    ConnectionStopped(ConnectionOutcome),

    #[error("Reached connection channel max")]
    ChannelMaxReached,
}
