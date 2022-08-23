//! Provide SASL-SCRAM acceptor

use std::sync::Arc;

use fe2o3_amqp_types::{primitives::{Array, Symbol, Binary}, sasl::{SaslOutcome, SaslCode, SaslChallenge}};
use rand::Rng;

use crate::{auth::{scram::{ScramCredentialProvider, StoredPassword, ScramVersion, DEFAULT_SCRAM_ITERATIONS, server::ScramAuthenticator}, error::ServerScramErrorKind}, sasl_profile::{SCRAM_SHA_1, SCRAM_SHA_256, SCRAM_SHA_512}};

use super::{SaslAcceptor, sasl_acceptor::SaslServerFrame};

/// Single SCRAM credential
#[derive(Debug)]
pub struct SingleScramCredential {
    username: String,
    salt: Vec<u8>,
    iterations: u32,
    stored_key: Vec<u8>,
    server_key: Vec<u8>,
}

impl SingleScramCredential {
    /// Creates a new [`SingleScramCredential`] with the specified [`ScramVersion`]
    pub fn new(
        username: impl Into<String>, 
        password: impl AsRef<str>,
        scram_version: &ScramVersion,
    ) -> Result<Self, ServerScramErrorKind> {
        let salt: [u8; 32] = rand::thread_rng().gen();
        let salt = Vec::from(salt);
        let salted_password = scram_version.compute_salted_password(password.as_ref(), &salt, DEFAULT_SCRAM_ITERATIONS)?;

        let client_key = scram_version.hmac(&salted_password, b"Client Key")?;
        let stored_key = scram_version.h(&client_key);
        let server_key = scram_version.hmac(&salted_password, b"Server Key")?;
        Ok(Self {
            username: username.into(),
            salt,
            iterations: DEFAULT_SCRAM_ITERATIONS,
            stored_key,
            server_key,
        })
    }
}

impl ScramCredentialProvider for SingleScramCredential {
    fn get_stored_password<'a>(&'a self, username: &str) -> Option<crate::auth::scram::StoredPassword<'a>> {
        if username == self.username {
            Some(StoredPassword {
                salt: &self.salt,
                iterations: self.iterations,
                stored_key: &self.stored_key,
                server_key: &self.server_key,
            })
        } else {
            None
        }
    }
}

/// Provide SASL-SCRAM-SHA-1 authentication
#[derive(Debug, Clone)]
pub struct SaslScramSha1Mechanism<C: ScramCredentialProvider> {
    scram_authenticator: ScramAuthenticator<C>
}

impl<C: ScramCredentialProvider + Clone> SaslScramSha1Mechanism<C> {
    /// Creates a new SASL-SCRAM-SHA-1 authentication mechanism
    pub fn new(credentials: C) -> Self {
        let scram_authenticator = ScramAuthenticator::new(credentials, DEFAULT_SCRAM_ITERATIONS, ScramVersion::Sha1);
        Self {
            scram_authenticator,
        }
    }
}

impl SaslScramSha1Mechanism<Arc<SingleScramCredential>> {
    /// Creates a SASL-SCRAM-SHA-1 authentication with a single credential
    pub fn single_credential(
        username: impl Into<String>, 
        password: impl AsRef<str>,
    ) -> Result<Self, ServerScramErrorKind> {
        let version = ScramVersion::Sha1;
        let credential = SingleScramCredential::new(username, password, &version)?;
        Ok(Self::new(Arc::new(credential)))
    }
}

/// Provide SASL-SCRAM-SHA-1 authentication
#[derive(Debug, Clone)]
pub struct SaslScramSha256Mechanism<C: ScramCredentialProvider> {
    scram_authenticator: ScramAuthenticator<C>
}

impl<C: ScramCredentialProvider + Clone> SaslScramSha256Mechanism<C> {
    /// Creates a new SASL-SCRAM-SHA-1 authentication mechanism
    pub fn new(credentials: C) -> Self {
        let scram_authenticator = ScramAuthenticator::new(credentials, DEFAULT_SCRAM_ITERATIONS, ScramVersion::Sha256);
        Self {
            scram_authenticator,
        }
    }
}

impl SaslScramSha256Mechanism<Arc<SingleScramCredential>> {
    /// Creates a SASL-SCRAM-SHA-1 authentication with a single credential
    pub fn single_credential(
        username: impl Into<String>, 
        password: impl AsRef<str>,
    ) -> Result<Self, ServerScramErrorKind> {
        let version = ScramVersion::Sha256;
        let credential = SingleScramCredential::new(username, password, &version)?;
        Ok(Self::new(Arc::new(credential)))
    }
}

/// Provide SASL-SCRAM-SHA-1 authentication
#[derive(Debug, Clone)]
pub struct SaslScramSha512Mechanism<C: ScramCredentialProvider> {
    scram_authenticator: ScramAuthenticator<C>
}

impl<C: ScramCredentialProvider + Clone> SaslScramSha512Mechanism<C> {
    /// Creates a new SASL-SCRAM-SHA-1 authentication mechanism
    pub fn new(credentials: C) -> Self {
        let scram_authenticator = ScramAuthenticator::new(credentials, DEFAULT_SCRAM_ITERATIONS, ScramVersion::Sha512);
        Self {
            scram_authenticator,
        }
    }
}

impl SaslScramSha512Mechanism<Arc<SingleScramCredential>> {
    /// Creates a SASL-SCRAM-SHA-1 authentication with a single credential
    pub fn single_credential(
        username: impl Into<String>, 
        password: impl AsRef<str>,
    ) -> Result<Self, ServerScramErrorKind> {
        let version = ScramVersion::Sha512;
        let credential = SingleScramCredential::new(username, password, &version)?;
        Ok(Self::new(Arc::new(credential)))
    }
}

macro_rules! impl_sasl_acceptor {
    ($mechanism_ty:ident, $mechanism:ident) => {
        impl<C> SaslAcceptor for $mechanism_ty<C> 
        where
            C: ScramCredentialProvider + Clone,
        {
            fn mechanisms(&self) -> fe2o3_amqp_types::primitives::Array<fe2o3_amqp_types::primitives::Symbol> {
                Array::from(vec![Symbol::from($mechanism)])
            }

            fn on_init(&mut self, init: fe2o3_amqp_types::sasl::SaslInit) -> SaslServerFrame {
                if init.mechanism.as_str() == $mechanism {
                    let client_first = match init.initial_response {
                        Some(client_first) => client_first,
                        None => return SaslServerFrame::Outcome(SaslOutcome {
                            code: SaslCode::Auth,
                            additional_data: None,
                        }),
                    };
                    match self.scram_authenticator.compute_server_first_message(&client_first) {
                        Ok(Some(server_first)) => {
                            SaslServerFrame::Challenge(SaslChallenge {
                                challenge: Binary::from(server_first),
                            })
                        },
                        Ok(None) | Err(_) => SaslServerFrame::Outcome(SaslOutcome {
                            code: SaslCode::Auth,
                            additional_data: None,
                        }),
                    }
                } else {
                    let outcome = SaslOutcome {
                        code: SaslCode::Auth,
                        additional_data: None,
                    };
                    SaslServerFrame::Outcome(outcome)
                }
            }

            fn on_response(&mut self, response: fe2o3_amqp_types::sasl::SaslResponse) -> SaslServerFrame {
                match self.scram_authenticator.compute_server_final_message(&response.response) {
                    Ok(server_final) => match server_final {
                        Some(server_final_message) => SaslServerFrame::Outcome(SaslOutcome {
                            code: SaslCode::Ok,
                            additional_data: Some(Binary::from(server_final_message)),
                        }),
                        // None means user is not found
                        None => SaslServerFrame::Outcome(SaslOutcome {
                            code: SaslCode::Auth,
                            additional_data: None,
                        }),
                    },
                    Err(_) => SaslServerFrame::Outcome(SaslOutcome {
                        code: SaslCode::Auth,
                        additional_data: None,
                    }),
                }
            }
        }
    };
}

impl_sasl_acceptor!(SaslScramSha1Mechanism, SCRAM_SHA_1);
impl_sasl_acceptor!(SaslScramSha256Mechanism, SCRAM_SHA_256);
impl_sasl_acceptor!(SaslScramSha512Mechanism, SCRAM_SHA_512);