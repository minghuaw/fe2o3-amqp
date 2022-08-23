#[cfg(feature = "scram")]
use crate::auth::error::ScramErrorKind;

/// SASL profile error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// AMQP error: not implemented
    #[error("Not implemented {0:?}")]
    NotImplemented(Option<String>),

    /// Error with SCRAM
    #[cfg_attr(docsrs, doc(cfg(feature = "scram")))]
    #[cfg(feature = "scram")]
    #[error(transparent)]
    ScramError(#[from] ScramErrorKind),
}
