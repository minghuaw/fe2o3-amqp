//! Trait definition(s) for authentication

/// Provide credential
pub trait CredentialProvider {
    /// Get the password if user exists
    fn get(&self, username: &str) -> Option<&str>;
}
