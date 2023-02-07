//! Implements SCRAM for SASL-SCRAM-SHA-1 and SASL-SCRAM-SHA-256 auth

pub(crate) mod error;

pub(crate) mod plain;

cfg_scram!{
    pub mod scram;
}

pub use plain::PlainCredentialProvider;
