//! Implements SCRAM for SASL-SCRAM-SHA-1 and SASL-SCRAM-SHA-256 auth

pub(crate) mod error;
pub(crate) mod scram;

/// Provide credential
pub trait CredentialProvider {
    /// Get the password if user exists
    fn get(&self, username: &str) -> Option<&str>;
}

