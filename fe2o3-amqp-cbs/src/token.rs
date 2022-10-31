use time::OffsetDateTime;

pub struct CbsToken {
    token_value: String,
    token_type: String,
    expires_at_utc: OffsetDateTime,
}

impl CbsToken {
    pub fn new(token_value: String, token_type: String, expires_at_utc: OffsetDateTime) -> Self {
        Self {
            token_value,
            token_type,
            expires_at_utc,
        }
    }

    pub fn token_value(&self) -> &str {
        &self.token_value
    }

    pub fn token_type(&self) -> &str {
        &self.token_type
    }

    pub fn expires_at_utc(&self) -> &OffsetDateTime {
        &self.expires_at_utc
    }
}