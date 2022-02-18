use std::io;

use fe2o3_amqp_types::definitions::{AmqpError, ConnectionError};

use crate::frames;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO Error {0:?}")]
    Io(#[from] io::Error),

    #[error("Idle timeout")]
    IdleTimeout,

    #[error("AMQP error {:?}, {:?}", .condition, .description)]
    AmqpError {
        condition: AmqpError,
        description: Option<String>,
    },

    #[error("Connection error: framing error")]
    FramingError,

    // #[error("Connection error {:?}, {:?}", .condition, .description)]
    // ConnectionError {
    //     condition: ConnectionError,
    //     description: Option<String>,
    // },
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

    // pub fn connection_error(
    //     condition: impl Into<ConnectionError>,
    //     description: impl Into<Option<String>>,
    // ) -> Self {
    //     Self::ConnectionError {
    //         condition: condition.into(),
    //         description: description.into(),
    //     }
    // }
}

/// TODO: What about encode error?
impl From<serde_amqp::Error> for Error {
    fn from(err: serde_amqp::Error) -> Self {
        match err {
            serde_amqp::Error::Io(e) => Self::Io(e),
            e @ _ => {
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
                description: None
            },
            frames::Error::FramingError => Self::FramingError,
        }
    }
}
