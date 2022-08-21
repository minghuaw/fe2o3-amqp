//! SASL-SCRAM-SHA1, SASL-SCRAM-SHA256

use crate::scram::{ScramClient, ScramVersion};

/// SASL-SCRAM-SHA-1
#[derive(Debug, Clone)]
pub struct SaslScramSha1 {
    pub(crate) client: ScramClient
}

impl SaslScramSha1 {
    /// Creates a [`SaslScramSha1`]
    pub fn new(username: String, password: String) -> Self {
        let client = ScramClient::new(username, password, ScramVersion::Sha1);
        Self {
            client,
        }
    }
}

/// SASL-SCRAM-SHA-256
#[derive(Debug, Clone)]
pub struct SaslScramSha256 {
    pub(crate) client: ScramClient
}

impl SaslScramSha256 {
    /// Creates a [`SaslScramSha1`]
    pub fn new(username: String, password: String) -> Self {
        let client = ScramClient::new(username, password, ScramVersion::Sha256);
        Self {
            client,
        }
    }
}
