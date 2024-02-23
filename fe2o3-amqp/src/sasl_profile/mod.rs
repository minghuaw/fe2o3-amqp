//! Implements SASL profile

use bytes::BufMut;
use fe2o3_amqp_types::{
    primitives::{Binary, Symbol},
    sasl::{SaslInit, SaslOutcome, SaslResponse},
};
use url::Url;

use crate::frames::sasl;

mod error;
pub use error::Error;

cfg_scram! {
    use crate::auth::error::ScramErrorKind;

    pub mod scram;

    pub use self::scram::{SaslScramSha1, SaslScramSha256, SaslScramSha512};

    pub(crate) const SCRAM_SHA_1: &str = "SCRAM-SHA-1";

    pub(crate) const SCRAM_SHA_256: &str = "SCRAM-SHA-256";

    pub(crate) const SCRAM_SHA_512: &str = "SCRAM-SHA-512";
}

// pub const EXTERN: Symbol = Symbol::from("EXTERNAL");
pub(crate) const ANONYMOUS: &str = "ANONYMOUS";
pub(crate) const PLAIN: &str = "PLAIN";

#[cfg_attr(not(feature = "scram"), allow(dead_code))]
pub(crate) enum Negotiation {
    Init(SaslInit),
    Response(SaslResponse),
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

    /// SASL-SCRAM-SHA-1
    #[cfg_attr(docsrs, doc(cfg(feature = "scram")))]
    #[cfg(feature = "scram")]
    ScramSha1(SaslScramSha1),

    /// SASL-SCRAM-SHA-256
    #[cfg_attr(docsrs, doc(cfg(feature = "scram")))]
    #[cfg(feature = "scram")]
    ScramSha256(SaslScramSha256),

    /// SASL-SCRAM-SHA-512
    #[cfg_attr(docsrs, doc(cfg(feature = "scram")))]
    #[cfg(feature = "scram")]
    ScramSha512(SaslScramSha512),
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
        let username = match value.username() {
            "" => return Err(()),
            username => username,
        };

        // Lazily evaluate value.password() if username is err
        let password = match value.password() {
            Some(password) => password,
            None => return Err(()),
        };

        Ok(SaslProfile::Plain {
            username: username.to_string(),
            password: password.to_string(),
        })
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
            #[cfg(feature = "scram")]
            SaslProfile::ScramSha1(_) => SCRAM_SHA_1,
            #[cfg(feature = "scram")]
            SaslProfile::ScramSha256(_) => SCRAM_SHA_256,
            #[cfg(feature = "scram")]
            SaslProfile::ScramSha512(_) => SCRAM_SHA_512,
        };
        Symbol::from(value)
    }

    pub(crate) fn initial_response(&mut self) -> Option<Binary> {
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
                Some(Binary::from(buf))
            }
            #[cfg(feature = "scram")]
            SaslProfile::ScramSha1(scram_sha1) => Some(Binary::from(
                scram_sha1.client.compute_client_first_message().to_vec(),
            )),
            #[cfg(feature = "scram")]
            SaslProfile::ScramSha256(scram_sha256) => Some(Binary::from(
                scram_sha256.client.compute_client_first_message().to_vec(),
            )),
            #[cfg(feature = "scram")]
            SaslProfile::ScramSha512(scram_sha512) => Some(Binary::from(
                scram_sha512.client.compute_client_first_message().to_vec(),
            )),
        }
    }

    /// How a SASL profile should respond to a SASL frame
    #[cfg_attr(not(feature = "scram"), allow(unused_variables))]
    pub(crate) fn on_frame(
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
            Frame::Challenge(challenge) => match self {
                SaslProfile::Anonymous | SaslProfile::Plain { .. } => Err(Error::NotImplemented(
                    Some("SASL Challenge is not implemented for ANONYMOUS or PLAIN.".to_string()),
                )),
                #[cfg(feature = "scram")]
                SaslProfile::ScramSha1(SaslScramSha1 { client })
                | SaslProfile::ScramSha256(SaslScramSha256 { client })
                | SaslProfile::ScramSha512(SaslScramSha512 { client }) => {
                    let server_first = std::str::from_utf8(&challenge.challenge)
                        .map_err(ScramErrorKind::Utf8Error)?;
                    let client_final = client.compute_client_final_message(server_first)?;
                    let response = SaslResponse {
                        response: Binary::from(client_final),
                    };

                    Ok(Negotiation::Response(response))
                }
            },
            Frame::Outcome(outcome) => {
                match self {
                    SaslProfile::Anonymous | SaslProfile::Plain { .. } => {}
                    #[cfg(feature = "scram")]
                    SaslProfile::ScramSha1(SaslScramSha1 { client })
                    | SaslProfile::ScramSha256(SaslScramSha256 { client })
                    | SaslProfile::ScramSha512(SaslScramSha512 { client }) => {
                        if matches!(outcome.code, fe2o3_amqp_types::sasl::SaslCode::Ok) {
                            let server_final = outcome
                                .additional_data
                                .as_ref()
                                .ok_or(ScramErrorKind::ServerSignatureMismatch)?;
                            client.validate_server_final(server_final)?;
                        }
                    }
                }
                Ok(Negotiation::Outcome(outcome))
            }
            _ => Err(Error::NotImplemented(Some(format!(
                "{:?} is not expected on client SASL profile",
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
        let mut profile = SaslProfile::Plain {
            username: String::from("user"),
            password: String::from("example"),
        };
        let _response = profile.initial_response();
    }
}
