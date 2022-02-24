use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO Error {0:?}")]
    Io(#[from] io::Error),

    #[error("Decode Error")]
    DecodeError,

    #[error("AmqpError: NotImplemented")]
    NotImplemented,
    // #[error("Framing Error")]
    // FramingError,
}

/// TODO: What about encode error?
impl From<serde_amqp::Error> for Error {
    fn from(err: serde_amqp::Error) -> Self {
        match err {
            serde_amqp::Error::Io(e) => Self::Io(e),
            _ => Self::DecodeError,
            // e @ _ => {
            //     let description = e.to_string();
            //     Self::AmqpError {
            //         condition: AmqpError::DecodeError,
            //         description: Some(description),
            //     }
            // }
        }
    }
}
