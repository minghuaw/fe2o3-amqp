use bytes::BufMut;
use fe2o3_amqp_types::{
    definitions::AmqpError,
    primitives::{Binary, Symbol},
    sasl::{SaslInit, SaslOutcome},
};
use serde_bytes::ByteBuf;
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
    Plain { username: String, password: String },
}

impl<'a> TryFrom<&'a Url> for SaslProfile {
    type Error = ();

    fn try_from(value: &'a Url) -> Result<Self, Self::Error> {
        match (value.username(), value.password()) {
            ("", _) | (_, None) => Err(()),
            (username, Some(password)) => Ok(SaslProfile::Plain {
                username: username.to_string(),
                password: password.to_string(),
            }),
        }
    }
}

impl SaslProfile {
    pub fn mechanism(&self) -> Symbol {
        let value = match self {
            SaslProfile::Anonymous => ANONYMOUS,
            SaslProfile::Plain {
                username: _,
                password: _,
            } => PLAIN,
        };
        Symbol::from(value)
    }

    pub fn initial_response(&self) -> Option<Binary> {
        match self {
            SaslProfile::Anonymous => None,
            SaslProfile::Plain { username, password } => {
                let username = username.as_bytes();
                let password = password.as_bytes();
                let mut buf = Vec::with_capacity(username.len() + password.len() + 2);
                buf.put_u8(0);
                buf.put_slice(username);
                buf.put_u8(0);
                buf.put_slice(password);
                Some(ByteBuf::from(buf))
            }
        }
    }

    pub async fn on_frame(
        &mut self,
        frame: sasl::Frame,
        hostname: Option<&str>,
    ) -> Result<Negotiation, Error> {
        use sasl::Frame;

        match frame {
            Frame::Mechanisms(mechanisms) => {
                let mechanism = self.mechanism();
                if mechanisms.sasl_server_mechanisms.contains(&mechanism) {
                    let init = SaslInit {
                        mechanism,
                        initial_response: self.initial_response(),
                        hostname: hostname.map(Into::into),
                    };
                    Ok(Negotiation::Init(init))
                } else {
                    Err(Error::AmqpError {
                        condition: AmqpError::NotImplemented,
                        description: Some(format!("{:?} is not supported", mechanism)),
                    })
                }
            }
            Frame::Challenge(challenge) => {
                todo!()
            }
            Frame::Outcome(outcome) => Ok(Negotiation::Outcome(outcome)),
            _ => Err(Error::AmqpError {
                condition: AmqpError::NotImplemented,
                description: Some(format!("{:?} is not expected", frame)),
            }),
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

    #[test]
    fn test_plain_initial_response() {
        let profile = SaslProfile::Plain {
            username: String::from("user"),
            password: String::from("example"),
        };
        let response = profile.initial_response();
        println!("{:?}", response);
    }
}
