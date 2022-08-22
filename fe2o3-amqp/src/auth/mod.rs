//! Implements SCRAM for SASL-SCRAM-SHA-1 and SASL-SCRAM-SHA-256 auth

pub(crate) mod error;

pub(crate) mod plain;

#[cfg(feature = "scram")]
pub(crate) mod scram;

pub use plain::PlainCredentialProvider;
