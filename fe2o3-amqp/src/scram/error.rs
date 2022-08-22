use std::str::Utf8Error;

use base64::DecodeError;
use hmac::digest::InvalidLength;

pub(crate) struct XorLengthMismatch {}

/// Error with SCRAM
#[derive(Debug, thiserror::Error)]
pub enum ScramErrorKind {
    /// Parsing str error
    #[error(transparent)]
    Utf8Error(#[from] Utf8Error),

    /// InsufficientParts
    #[error("InsufficientParts")]
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
    #[error(transparent)]
    HmacErrorInvalidLength(#[from] InvalidLength),

    /// LHS and RHS of XOR have different length
    #[error("LHS and RHS of XOR have different length")]
    XorLengthMismatch,

    /// Illegal SCRAM client state
    #[error("Illegal SCRAM client state")]
    IllegalClientState,

    /// Server signature mismatch
    #[error("Server signature mismatch")]
    ServerSignatureMismatch,
}

impl From<XorLengthMismatch> for ScramErrorKind {
    fn from(_: XorLengthMismatch) -> Self {
        Self::XorLengthMismatch
    }
}

/// Server SASL-SCRAM authenticator errors
#[derive(Debug, thiserror::Error)]
pub(crate) enum ServerScramErrorKind {
    /// Parsing str error
    #[error(transparent)]
    Utf8Error(#[from] Utf8Error),

    /// Error parsing GS2-HEADER error
    #[error("Cannot parse GS2-header")]
    CannotParseGs2Header,

    /// Error parsing username
    #[error("Error parsing username field")]
    CannotParseUsername,

    /// Error parsing client nonce,
    #[error("Error parsing client nonce")]
    CannotParseClientNonce,

    /// Error from `Mac::new_from_slice`
    #[error(transparent)]
    HmacErrorInvalidLength(#[from] InvalidLength),

    /// LHS and RHS of XOR have different length
    #[error("LHS and RHS of XOR have different length")]
    XorLengthMismatch,

    /// Error normalizing password
    #[error("Error normalizing password")]
    NormalizeError(#[from] stringprep::Error),

    /// Error parsing client final message
    #[error("Error parsing client final message")]
    CannotParseClientFinalMessage,

    /// Invalid channel binding 
    #[error("Invalid channel binding")]
    InvalidChannelBinding,

    /// Error decoding base64 values
    #[error(transparent)]
    Base64DecodeError(#[from] DecodeError),

    /// Client final message has an incorrect nonce
    #[error("Client final message has an incorrect nonce")]
    IncorrectClientFinalNonce,

    /// Client final message doesn't have a proof
    #[error("Client final message doesnt' have a proof")]
    ProofNotFoundInClientFinal,

    /// Authentication failed
    #[error("Authentication failed")]
    AuthenticationFailed,

    /// Illegal SCRAM authenticator state
    #[error("Illegal SCRAM client state")]
    IllegalAuthenticatorState,
}

impl From<XorLengthMismatch> for ServerScramErrorKind {
    fn from(_: XorLengthMismatch) -> Self {
        Self::XorLengthMismatch
    }
}