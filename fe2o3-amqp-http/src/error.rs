use http::header::ToStrError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectedModeError<B> {
    #[error(transparent)]
    ToStrError(#[from] ToStrError),

    #[error(transparent)]
    Date(#[from] httpdate::Error),

    #[error(transparent)]
    SystemTime(#[from] std::time::SystemTimeError),

    #[error(transparent)]
    Body(B),
}