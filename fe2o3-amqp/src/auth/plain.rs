
/// Provide credential
pub trait PlainCredentialProvider {
    /// Get the password if user exists
    fn get(&self, username: &str) -> Option<&str>;
}
