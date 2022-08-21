//! SASL-SCRAM-SHA-11, SASL-SCRAM-SHA-256, and SASL-SCRAM-SHA-512

use crate::scram::{ScramClient, ScramVersion};

use super::SaslProfile;

/// SASL-SCRAM-SHA-1
///
/// The SHA-1 hash function should be considered cryptographically
/// broken and unsuitable for further use in any security critical capacity,
/// as it is practically vulnerable to chosen-prefix collisions.
#[derive(Debug, Clone)]
pub struct SaslScramSha1 {
    pub(crate) client: ScramClient,
}

impl SaslScramSha1 {
    /// Creates a [`SaslScramSha1`]
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        let client = ScramClient::new(username, password, ScramVersion::Sha1);
        Self { client }
    }
}

impl From<SaslScramSha1> for SaslProfile {
    fn from(sha1: SaslScramSha1) -> Self {
        Self::ScramSha1(sha1)
    }
}

/// SASL-SCRAM-SHA-256
#[derive(Debug, Clone)]
pub struct SaslScramSha256 {
    pub(crate) client: ScramClient,
}

impl SaslScramSha256 {
    /// Creates a [`SaslScramSha1`]
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        let client = ScramClient::new(username, password, ScramVersion::Sha256);
        Self { client }
    }
}

impl From<SaslScramSha256> for SaslProfile {
    fn from(sha1: SaslScramSha256) -> Self {
        Self::ScramSha256(sha1)
    }
}

/// SASL-SCRAM-SHA-512
#[derive(Debug, Clone)]
pub struct SaslScramSha512 {
    pub(crate) client: ScramClient,
}

impl SaslScramSha512 {
    /// Creates a [`SaslScramSha1`]
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        let client = ScramClient::new(username, password, ScramVersion::Sha512);
        Self { client }
    }
}

impl From<SaslScramSha512> for SaslProfile {
    fn from(sha1: SaslScramSha512) -> Self {
        Self::ScramSha512(sha1)
    }
}
