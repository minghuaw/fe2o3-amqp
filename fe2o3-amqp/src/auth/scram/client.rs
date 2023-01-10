use bytes::Bytes;

use super::{generate_nonce, ScramErrorKind, ScramVersion};

#[derive(Debug, Clone)]
enum ScramClientState {
    Initial,
    ClientFirstSent {
        client_nonce: String,
        client_first_message_bare: Bytes,
    },
    ClientFinalSent {
        server_signature: Vec<u8>,
    },
    Complete,
}

#[derive(Debug, Clone)]
pub(crate) struct ScramClient {
    username: String,
    password: String,
    scram: ScramVersion,
    state: ScramClientState,
}

impl ScramClient {
    pub fn new(
        username: impl Into<String>,
        password: impl Into<String>,
        scram_version: ScramVersion,
    ) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            scram: scram_version,
            state: ScramClientState::Initial,
        }
    }

    pub fn compute_client_first_message(&mut self) -> Bytes {
        use base64::Engine;

        let nonce = base64::engine::general_purpose::STANDARD.encode(generate_nonce());
        let (client_first_message, client_first_message_bare) = self
            .scram
            .client_first_message(self.username.as_bytes(), nonce.as_bytes());
        self.state = ScramClientState::ClientFirstSent {
            client_nonce: nonce,
            client_first_message_bare,
        };
        client_first_message
    }

    pub fn compute_client_final_message(
        &mut self,
        server_first: &str,
    ) -> Result<Vec<u8>, ScramErrorKind> {
        match &self.state {
            ScramClientState::ClientFirstSent {
                client_nonce,
                client_first_message_bare,
            } => {
                let (client_final, server_signature) = self.scram.compute_client_final_message(
                    client_nonce,
                    &self.password,
                    server_first,
                    client_first_message_bare,
                )?;
                self.state = ScramClientState::ClientFinalSent { server_signature };
                Ok(client_final)
            }
            _ => Err(ScramErrorKind::IllegalClientState),
        }
    }

    pub fn validate_server_final(&mut self, server_final: &[u8]) -> Result<(), ScramErrorKind> {
        match &self.state {
            ScramClientState::ClientFinalSent { server_signature } => {
                self.scram
                    .validate_server_final(server_final, server_signature)?;
                self.state = ScramClientState::Complete;
                Ok(())
            }
            _ => Err(ScramErrorKind::IllegalClientState),
        }
    }
}
