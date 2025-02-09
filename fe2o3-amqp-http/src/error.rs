use http::header::ToStrError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectedModeError {
    #[error(transparent)]
    ToStrError(#[from] ToStrError),

    #[error(transparent)]
    Date(#[from] httpdate::Error),

    #[error(transparent)]
    SystemTime(#[from] std::time::SystemTimeError),
}