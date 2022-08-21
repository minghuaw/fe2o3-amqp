use std::str::Utf8Error;

use base64::DecodeError;


/// Error with SCRAM
#[derive(Debug, thiserror::Error)]
pub enum ScramErrorKind {
    /// Parsing str error
    #[error(transparent)]
    Utf8Error(#[from] Utf8Error),

    /// Less than 3 parts are found in challenge
    #[error("Less 3 parts are found in challenge")]
    InsufficientParts,

    /// Found "m=" in challenge, which is not supported
    #[error("\"m=\" is not supported")]
    ExtensionNotSupported,

    /// Cannot find Nonce in challenge
    #[error("Cannot find Nonce in challenge")]
    NonceNotFound,

    /// Server Nonce should start with client Nonce
    #[error("Server Nonce should start with client Nonce")]
    ClientNonceMismatch,

    /// Salt key is not found in challenge
    #[error("Salt key is not found in challenge")]
    SaltNotFound,

    /// Error decoding base64 values
    #[error(transparent)]
    Base64DecodeError(#[from] DecodeError),

    /// Iteration count is not found in challenge
    #[error("Iteration count is not found in challenge")]
    IterationCountNotFound,

    /// Cannot parse iteration count
    #[error("Cannot parse iteration count")]
    IterationCountParseError,

    /// Error normalizing password
    #[error("Error normalizing password")]
    NormalizeError(#[from] stringprep::Error),

    /// Error from `Mac::new_from_slice`
    #[error("Error from `Mac::new_from_slice`")]
    HmacErrorInvalidLength,

    /// LHS and RHS of XOR have different length
    #[error("LHS and RHS of XOR have different length")]
    XorLengthMismatch,

    /// Illegal SCRAM client state
    #[error("Illegal SCRAM client state")]
    IllegalClientState,
}