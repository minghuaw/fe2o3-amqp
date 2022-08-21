use crate::scram::error::ScramErrorKind;

/// SASL profile error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// AMQP error: not implemented
    #[error("Not implemented {0:?}")]
    NotImplemented(Option<String>),

    /// Error with SCRAM
    #[error(transparent)]
    ScramError(#[from] ScramErrorKind),
}
