use std::borrow::Cow;

use time::OffsetDateTime;

pub struct CbsToken<'a> {
    name: Cow<'a, str>,
    token_value: Cow<'a, str>,
    token_type: Cow<'a, str>,
    expires_at_utc: Option<OffsetDateTime>,
}

impl<'a> CbsToken<'a> {
    pub fn new(
        name: impl Into<Cow<'a, str>>,
        token_value: impl Into<Cow<'a, str>>,
        token_type: impl Into<Cow<'a, str>>,
        expires_at_utc: impl Into<Option<OffsetDateTime>>,
    ) -> Self {
        Self {
            name: name.into(),
            token_value: token_value.into(),
            token_type: token_type.into(),
            expires_at_utc: expires_at_utc.into(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn token_value(&self) -> &str {
        &self.token_value
    }

    pub fn token_type(&self) -> &str {
        &self.token_type
    }

    pub fn expires_at_utc(&self) -> &Option<OffsetDateTime> {
        &self.expires_at_utc
    }
}
