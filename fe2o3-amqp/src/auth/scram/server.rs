use bytes::Bytes;
use rand::Rng;

use super::{error::ServerScramErrorKind, generate_nonce, ScramVersion, ScramCredentialProvider};

#[derive(Debug, Clone)]
enum ScramAuthenticatorState {
    Initial,
    ServerFirstSent {
        salt: [u8; 32],
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
    iterations: u32,
    scram: ScramVersion,
    state: ScramAuthenticatorState,
}

impl<C> ScramAuthenticator<C>
where
    C: ScramCredentialProvider,
{
    pub fn new(credentials: C, iterations: u32, scram_version: ScramVersion) -> Self {
        Self {
            credentials,
            iterations,
            scram: scram_version,
            state: ScramAuthenticatorState::Initial,
        }
    }

    pub fn compute_server_first_message(
        &mut self,
        client_first_message: &[u8],
    ) -> Result<Option<impl Into<Vec<u8>>>, ServerScramErrorKind> {
        let salt: [u8; 32] = rand::thread_rng().gen();
        let base64_salt = base64::encode(salt.clone());
        let nonce = generate_nonce();
        let base64_server_nonce = base64::encode(nonce);

        let server_first = self.scram.compute_server_first_message(
            client_first_message,
            &base64_salt,
            &base64_server_nonce,
            self.iterations,
        )?;

        self.state = ScramAuthenticatorState::ServerFirstSent {
            salt,
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
        // match &self.state {
        //     ScramAuthenticatorState::ServerFirstSent {
        //         salt,
        //         username,
        //         client_first_message_bare,
        //         client_server_nonce,
        //         server_first_message,
        //     } => {
        //         // look up user
        //         let salted_password = match self.credentials.get(username) {
        //             Some(password) => {
        //                 self.scram
        //                     .compute_salted_password(password, &salt[..], self.iterations)?
        //             }
        //             None => {
        //                 // User is not found
        //                 self.state = ScramAuthenticatorState::Initial;
        //                 return Ok(None);
        //             }
        //         };

        //         let server_final_message = self.scram.compute_server_final_message(
        //             client_final_message,
        //             &client_server_nonce,
        //             &client_first_message_bare,
        //             &server_first_message,
        //             &salted_password,
        //         )?;
        //         self.state = ScramAuthenticatorState::ServerFinalSent;
        //         Ok(Some(server_final_message))
        //     }
        //     _ => Err(ServerScramErrorKind::IllegalAuthenticatorState),
        // }
        todo!()
    }
}
