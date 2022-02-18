use std::os::windows::prelude::AsRawHandle;

use fe2o3_amqp_types::{primitives::Symbol, sasl::{SaslOutcome, SaslInit}, definitions::AmqpError};
use futures_util::Sink;
use url::Url;

pub mod error;

pub use error::Error;

use crate::frames::sasl;

// pub const EXTERN: Symbol = Symbol::from("EXTERNAL");
pub const ANONYMOUS: &str = "ANONYMOUS";
pub const PLAIN: &str = "PLAIN";

pub enum Negotiation {
    Continue,
    Init(SaslInit),
    Outcome(SaslOutcome),
}

#[derive(Debug, Clone)]
pub enum SaslProfile {
    Anonymous,
    Plain {
        username: String,
        password: String,
    }
}

impl<'a> TryFrom<&'a Url> for SaslProfile {
    type Error = &'a Url;

    fn try_from(value: &'a Url) -> Result<Self, Self::Error> {
        match (value.username(), value.password()){
            ("", _) | (_, None) => Err(value),
            (username, Some(password)) => {
                Ok(SaslProfile::Plain {
                    username: username.to_string(),
                    password: password.to_string()
                })
            }
        }
    }
}

impl SaslProfile {
    pub fn mechanism(&self) -> Symbol {
        let value = match self {
            SaslProfile::Anonymous => ANONYMOUS,
            SaslProfile::Plain {username: _, password: _} => PLAIN,
        };
        Symbol::from(value)
    }

    pub async fn on_frame<W>(&mut self, frame: sasl::Frame, hostname: Option<String>) -> Result<Negotiation, Error> {
        use sasl::Frame;

        match frame {
            Frame::Mechanisms(mechanisms) => {
                let mechanism = self.mechanism();
                if mechanisms.sasl_server_mechanisms.contains(&mechanism) {
                    let init = SaslInit {
                        mechanism,
                        initial_response: None,
                        hostname,
                    };
                    Ok(Negotiation::Init(init))
                } else {
                    Err(Error::AmqpError{
                        condition: AmqpError::NotImplemented,
                        description: Some(format!("{:?} is not supported", mechanism))
                    })
                }
            },
            Frame::Challenge(challenge) => {
                todo!()
            },
            Frame::Outcome(outcome) => {
                Ok(Negotiation::Outcome(outcome))
            },
            _ => Err(Error::AmqpError {
                condition: AmqpError::NotImplemented,
                description: Some(format!("{:?} is not expected", frame))
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use url::Url;

    use super::SaslProfile;

    #[test]
    fn test_try_from_address() {
        let url = "amqps://username:password@example.com";
        let url = Url::try_from(url).unwrap();
        let result = SaslProfile::try_from(&url);
        assert!(result.is_ok());

        let url = "amqps://:password@example.com";
        let url = Url::try_from(url).unwrap();
        let result = SaslProfile::try_from(&url);
        assert!(result.is_err());

        let url = "amqps://username:@example.com";
        let url = Url::try_from(url).unwrap();
        let result = SaslProfile::try_from(&url);
        assert!(result.is_err());

        let url = "amqps://@example.com";
        let url = Url::try_from(url).unwrap();
        let result = SaslProfile::try_from(&url);
        assert!(result.is_err());
    }
}