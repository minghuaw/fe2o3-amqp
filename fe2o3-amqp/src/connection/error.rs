use std::io;

use fe2o3_amqp_types::definitions::{AmqpError, ConnectionError};

use crate::transport;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO Error {0:?}")]
    Io(#[from] io::Error),

    #[error("Idle timeout")]
    IdleTimeout,

    #[error(transparent)]
    UrlError(url::ParseError),

    #[error("Exceeding channel-max")]
    ChannelMaxExceeded,

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

impl From<transport::Error> for Error {
    fn from(err: transport::Error) -> Self {
        match err {
            transport::Error::Io(e) => Self::Io(e),
            transport::Error::IdleTimeout => Self::IdleTimeout,
            transport::Error::AmqpError {
                condition,
                description,
            } => Self::AmqpError {
                condition,
                description,
            },
            transport::Error::ConnectionError {
                condition,
                description,
            } => Self::ConnectionError {
                condition,
                description,
            },
        }
    }
}
