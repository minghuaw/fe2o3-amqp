use bytes::Bytes;

use super::{error::ServerScramErrorKind, generate_nonce, ScramVersion, ScramCredentialProvider};

#[derive(Debug, Clone)]
enum ScramAuthenticatorState {
    Initial,
    ServerFirstSent {
        username: String,
        client_first_message_bare: Bytes,
        client_server_nonce: Bytes,
        server_first_message: Bytes,
    },
    ServerFinalSent,
}

#[derive(Debug, Clone)]
pub(crate) struct ScramAuthenticator<C: ScramCredentialProvider> {
    credentials: C,
    scram: ScramVersion,
    state: ScramAuthenticatorState,
}

impl<C> ScramAuthenticator<C>
where
    C: ScramCredentialProvider,
{
    pub fn new(credentials: C, scram_version: ScramVersion) -> Self {
        Self {
            credentials,
            scram: scram_version,
            state: ScramAuthenticatorState::Initial,
        }
    }

    pub fn compute_server_first_message(
        &mut self,
        client_first_message: &[u8],
    ) -> Result<Option<impl Into<Vec<u8>>>, ServerScramErrorKind> {
        let nonce = generate_nonce();
        let base64_server_nonce = base64::encode(nonce);

        let server_first = match self.scram.compute_server_first_message(
            client_first_message,
            &base64_server_nonce,
            &self.credentials
        )? {
            Some(server_first) => server_first,
            None => return Ok(None),
        };

        self.state = ScramAuthenticatorState::ServerFirstSent {
            username: server_first.username.to_string(),
            client_first_message_bare: server_first.client_first_message_bare,
            client_server_nonce: server_first.client_server_nonce,
            server_first_message: server_first.message.clone(),
        };

        Ok(Some(server_first.message))
    }

    pub fn compute_server_final_message(
        &mut self,
        client_final_message: &[u8],
    ) -> Result<Option<impl Into<Vec<u8>>>, ServerScramErrorKind> {
        match &self.state {
            ScramAuthenticatorState::ServerFirstSent {
                username,
                client_first_message_bare,
                client_server_nonce,
                server_first_message,
            } => {
                // look up user
                let stored_password = match self.credentials.get_stored_password(username) {
                    Some(stored) => stored,
                    None => return Ok(None),
                };

                let server_final_message = self.scram.compute_server_final_message(
                    client_final_message,
                    &client_server_nonce,
                    &client_first_message_bare,
                    &server_first_message,
                    &stored_password,
                )?;
                self.state = ScramAuthenticatorState::ServerFinalSent;
                Ok(Some(server_final_message))
            }
            _ => Err(ServerScramErrorKind::IllegalAuthenticatorState),
        }
    }
}
