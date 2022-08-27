//! Provide SASL-SCRAM acceptor

use fe2o3_amqp_types::{
    primitives::{Array, Binary, Symbol},
    sasl::{SaslChallenge, SaslCode, SaslOutcome},
};
use rand::Rng;

use crate::auth::{
    error::ServerScramErrorKind,
    scram::{
        ScramAuthenticator, ScramCredentialProvider, ScramVersion, StoredPassword,
        DEFAULT_SCRAM_ITERATIONS,
    },
};

use super::{sasl_acceptor::SaslServerFrame, SaslAcceptor};

/// Single SCRAM credential
#[derive(Debug)]
pub struct SingleScramCredential {
    scram_version: ScramVersion,
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
        scram_version: ScramVersion,
    ) -> Result<Self, ServerScramErrorKind> {
        let salt: [u8; 32] = rand::thread_rng().gen();
        let salt = Vec::from(salt);
        let salted_password = scram_version.compute_salted_password(
            password.as_ref(),
            &salt,
            DEFAULT_SCRAM_ITERATIONS,
        )?;

        let client_key = scram_version.hmac(&salted_password, b"Client Key")?;
        let stored_key = scram_version.h(&client_key);
        let server_key = scram_version.hmac(&salted_password, b"Server Key")?;
        Ok(Self {
            scram_version,
            username: username.into(),
            salt,
            iterations: DEFAULT_SCRAM_ITERATIONS,
            stored_key,
            server_key,
        })
    }
}

impl ScramCredentialProvider for SingleScramCredential {
    fn scram_version(&self) -> &ScramVersion {
        &self.scram_version
    }

    fn get_stored_password<'a>(
        &'a self,
        username: &str,
    ) -> Option<crate::auth::scram::StoredPassword<'a>> {
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

impl<C> SaslAcceptor for ScramAuthenticator<C>
where
    C: ScramCredentialProvider + Clone,
{
    fn mechanisms(
        &self,
    ) -> fe2o3_amqp_types::primitives::Array<fe2o3_amqp_types::primitives::Symbol> {
        Array::from(vec![Symbol::from(
            self.credentials().scram_version().mechanism(),
        )])
    }

    fn on_init(&mut self, init: fe2o3_amqp_types::sasl::SaslInit) -> SaslServerFrame {
        if init.mechanism.as_str() == self.credentials().scram_version().mechanism() {
            let client_first = match init.initial_response {
                Some(client_first) => client_first,
                None => {
                    return SaslServerFrame::Outcome(SaslOutcome {
                        code: SaslCode::Auth,
                        additional_data: None,
                    })
                }
            };
            match self.compute_server_first_message(&client_first) {
                Ok(Some(server_first)) => SaslServerFrame::Challenge(SaslChallenge {
                    challenge: Binary::from(server_first),
                }),
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
        match self.compute_server_final_message(&response.response) {
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
