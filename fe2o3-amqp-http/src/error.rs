use http::{header::{InvalidHeaderValue, ToStrError}, method::InvalidMethod, status::InvalidStatusCode};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TryIntoProjectedError<BE> {
    #[error(transparent)]
    ToStrError(#[from] ToStrError),

    #[error(transparent)]
    Date(#[from] httpdate::Error),

    #[error(transparent)]
    SystemTime(#[from] std::time::SystemTimeError),

    #[error(transparent)]
    Body(BE),
}

#[derive(Debug, Error)]
pub enum TryFromProjectedError<BE> {
    #[error("HTTP method not present")]
    MethodNotPresent,

    #[error(transparent)]
    InvalidMethod(#[from] InvalidMethod),

    #[error("HTTP status not present")]
    StatusNotPresent,

    #[error(transparent)]
    InvalidStatusCode(#[from] InvalidStatusCode),

    #[error("HTTP request target not present")]
    RequestTargetNotPresent,

    #[error("HTTP version not encoded as a string")]
    VersionNotString,

    #[error("HTTP version not valid")]
    InvalidVersion,

    #[error(transparent)]
    HeaderValue(#[from] InvalidHeaderValue),

    #[error(transparent)]
    Body(BE),

    #[error(transparent)]
    Http(#[from] http::Error),
}

pub(crate) struct InvalidVersion;

impl<T> From<InvalidVersion> for TryFromProjectedError<T> {
    fn from(_: InvalidVersion) -> Self {
        TryFromProjectedError::InvalidVersion
    }
}