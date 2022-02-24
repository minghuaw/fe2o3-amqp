use std::io;

use fe2o3_amqp_types::{
    definitions::{AmqpError, ConnectionError, self, ErrorCondition},
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

    // #[error("Exceeding channel-max")]
    // ChannelMaxReached,

    // #[error("AMQP error {:?}, {:?}", .condition, .description)]
    // AmqpError {
    //     condition: AmqpError,
    //     description: Option<String>,
    // },

    // #[error("Connection error {:?}, {:?}", .condition, .description)]
    // ConnectionError {
    //     condition: ConnectionError,
    //     description: Option<String>,
    // },
    #[error("Local error {:?}", .0)]
    LocalError(definitions::Error),

    #[error("Remote error {:?}", .0)]
    RemoteError(definitions::Error)
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
        Self::LocalError(
            definitions::Error {
                condition: ErrorCondition::AmqpError(err),
                description: None,
                info: None
            }
        )
    }
}

impl Error {
    pub(crate) fn amqp_error(
        condition: impl Into<AmqpError>,
        description: impl Into<Option<String>>,
    ) -> Self {
        Self::LocalError(
            definitions::Error {
                condition: ErrorCondition::AmqpError(condition.into()),
                description: description.into(),
                info: None
            }
        )
    }

    pub(crate) fn connection_error(
        condition: impl Into<ConnectionError>,
        description: impl Into<Option<String>>,
    ) -> Self {
        Self::LocalError(
            definitions::Error {
                condition: ErrorCondition::ConnectionError(condition.into()),
                description: description.into(),
                info: None
            }
        )
    }
}

impl From<transport::Error> for Error {
    fn from(err: transport::Error) -> Self {
        match err {
            transport::Error::Io(e) => Self::Io(e),
            transport::Error::IdleTimeout => Self::connection_error(ConnectionError::ConnectionForced, Some("Idle timeout".to_string())),
            transport::Error::AmqpError {
                condition,
                description,
            } => Self::amqp_error(condition, description),
            transport::Error::FramingError => Self::connection_error(ConnectionError::FramingError, None),
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

    // #[error("AMQP error {:?}, {:?}", .condition, .description)]
    // AmqpError {
    //     condition: AmqpError,
    //     description: Option<String>,
    // },

    // #[error("Connection error {:?}, {:?}", .condition, .description)]
    // ConnectionError {
    //     condition: ConnectionError,
    //     description: Option<String>,
    // },

    #[error("SASL error code {:?}, additional data: {:?}", .code, .additional_data)]
    SaslError {
        code: SaslCode,
        additional_data: Option<Binary>,
    },

    #[error("Local error {:?}", .0)]
    LocalError(definitions::Error),

    #[error("Remote error {:?}", .0)]
    RemoteError(definitions::Error)
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
            NegotiationError::DecodeError => todo!(),
            NegotiationError::NotImplemented(_) => todo!(),
            NegotiationError::IllegalState => todo!(),
        }
    }
}

// impl From<AmqpError> for OpenError {
//     fn from(err: AmqpError) -> Self {
//         Self::AmqpError {
//             condition: err,
//             description: None,
//         }
//     }
// }
