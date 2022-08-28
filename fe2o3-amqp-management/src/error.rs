use fe2o3_amqp::link::{SenderAttachError, ReceiverAttachError};

#[derive(Debug, thiserror::Error)]
pub enum AttachError {
    #[error(transparent)]
    Sender(#[from] SenderAttachError),

    #[error(transparent)]
    Receiver(#[from] ReceiverAttachError),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {

}

pub type Result<T> = std::result::Result<T, Error>;
