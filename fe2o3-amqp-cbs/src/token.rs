//! Defines CbsToken struct

use std::borrow::Cow;

use fe2o3_amqp::types::primitives::Timestamp;

/// A CBS token
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CbsToken<'a> {
    pub(crate) token_value: Cow<'a, str>,
    pub(crate) token_type: Cow<'a, str>,
    pub(crate) expires_at_utc: Option<Timestamp>,
}

impl<'a> CbsToken<'a> {
    /// Create a new CBS token
    pub fn new(
        token_value: impl Into<Cow<'a, str>>,
        token_type: impl Into<Cow<'a, str>>,
        expires_at_utc: impl Into<Option<Timestamp>>,
    ) -> Self {
        Self {
            token_value: token_value.into(),
            token_type: token_type.into(),
            expires_at_utc: expires_at_utc.into(),
        }
    }

    /// Get the token value
    pub fn token_value(&self) -> &str {
        &self.token_value
    }

    /// Get the token type
    pub fn token_type(&self) -> &str {
        &self.token_type
    }

    /// Get the expiration time
    pub fn expires_at_utc(&self) -> &Option<Timestamp> {
        &self.expires_at_utc
    }
}
