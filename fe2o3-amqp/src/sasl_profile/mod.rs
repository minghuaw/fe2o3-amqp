//! Implements SASL profile

use bytes::BufMut;
use fe2o3_amqp_types::{
    primitives::{Binary, Symbol},
    sasl::{SaslInit, SaslOutcome, SaslResponse},
};
use serde_bytes::ByteBuf;
use url::Url;

mod error;
pub use error::Error;

use crate::frames::sasl;

// pub const EXTERN: Symbol = Symbol::from("EXTERNAL");
pub(crate) const ANONYMOUS: &str = "ANONYMOUS";
pub(crate) const PLAIN: &str = "PLAIN";

pub(crate) enum Negotiation {
    Init(SaslInit),
    _Response(SaslResponse),
    Outcome(SaslOutcome),
}

/// SASL profile
#[derive(Debug, Clone)]
pub enum SaslProfile {
    /// SASL profile for ANONYMOUS mechanism
    Anonymous,

    /// SASL profile for PLAIN mechanism
    Plain {
        /// Username
        username: String,
        /// Password
        password: String,
    },
}

impl<T1, T2> From<(T1, T2)> for SaslProfile
where
    T1: Into<String>,
    T2: Into<String>,
{
    fn from((username, password): (T1, T2)) -> Self {
        Self::Plain {
            username: username.into(),
            password: password.into(),
        }
    }
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
    pub(crate) fn mechanism(&self) -> Symbol {
        let value = match self {
            SaslProfile::Anonymous => ANONYMOUS,
            SaslProfile::Plain {
                username: _,
                password: _,
            } => PLAIN,
        };
        Symbol::from(value)
    }

    pub(crate) fn initial_response(&self) -> Option<Binary> {
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

    /// How a SASL profile should respond to a SASL frame
    pub(crate) async fn on_frame(
        &mut self,
        frame: sasl::Frame,
        hostname: Option<&str>,
    ) -> Result<Negotiation, Error> {
        use sasl::Frame;

        match frame {
            Frame::Mechanisms(mechanisms) => {
                let mechanism = self.mechanism();
                if mechanisms.sasl_server_mechanisms.0.contains(&mechanism) {
                    let init = SaslInit {
                        mechanism,
                        initial_response: self.initial_response(),
                        hostname: hostname.map(Into::into),
                    };
                    Ok(Negotiation::Init(init))
                } else {
                    Err(Error::NotImplemented(Some(format!(
                        "{:?} is not supported",
                        mechanism
                    ))))
                }
            }
            Frame::Challenge(_challenge) => {
                // TODO
                Err(Error::NotImplemented(Some(
                    "SASL Challenge is not implemented.".to_string(),
                )))
            }
            Frame::Outcome(outcome) => Ok(Negotiation::Outcome(outcome)),
            _ => Err(Error::NotImplemented(Some(format!(
                "{:?} is not expected",
                frame
            )))),
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
