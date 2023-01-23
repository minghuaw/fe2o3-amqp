use bytes::Bytes;

use super::*;

impl ScramVersion {
    fn compute_server_first_message<'a, C>(
        &self,
        client_first: &'a [u8],
        base64_server_nonce: &str,
        credentials: &C,
    ) -> Result<Option<ServerFirstMessage<'a>>, ServerScramErrorKind>
    where
        C: ScramCredentialProvider,
    {
        let client_first = std::str::from_utf8(client_first)?;

        let client_first_message_bare = client_first
            .strip_prefix(GS2_HEADER)
            .ok_or(ServerScramErrorKind::CannotParseGs2Header)?;

        let parts: Vec<&str> = client_first_message_bare.split(',').collect();

        let username = parts
            .first()
            .and_then(|s| s.strip_prefix(USERNAME_KEY))
            .ok_or(ServerScramErrorKind::CannotParseUsername)?;
        let client_nonce = parts
            .get(1)
            .and_then(|s| s.strip_prefix(NONCE_KEY))
            .ok_or(ServerScramErrorKind::CannotParseClientNonce)?;

        let stored_password = match credentials.get_stored_password(username) {
            Some(stored) => stored,
            None => return Ok(None),
        };
        let base64_salt = base64::encode(stored_password.salt);
        let iterations = stored_password.iterations.to_string();

        let client_server_nonce = format!("{}{}", client_nonce, base64_server_nonce);

        let mut buf = BytesMut::new();

        // nonce
        buf.put_slice(NONCE_KEY.as_bytes());
        buf.put_slice(client_server_nonce.as_bytes());
        buf.put_u8(b',');

        // salt
        buf.put_slice(SALT_KEY.as_bytes());
        buf.put_slice(base64_salt.as_bytes());
        buf.put_u8(b',');

        // iterations
        buf.put_slice(ITERATION_COUNT_KEY.as_bytes());
        buf.put_slice(iterations.as_bytes());

        Ok(Some(ServerFirstMessage {
            username,
            client_first_message_bare: Bytes::from(
                client_first_message_bare.to_string().into_bytes(),
            ),
            client_server_nonce: Bytes::from(client_server_nonce.into_bytes()),
            message: buf.freeze(),
        }))
    }

    fn compute_server_final_message(
        &self,
        client_final: &[u8],
        client_server_nonce: &[u8],
        client_first_message_bare: &[u8],
        server_first_message: &[u8],
        stored_password: &StoredPassword,
    ) -> Result<Vec<u8>, ServerScramErrorKind> {
        let client_final = std::str::from_utf8(client_final)?;
        let parts: Vec<&str> = client_final.split(',').collect();

        let channel_binding = parts
            .first()
            .and_then(|s| s.strip_prefix(CHANNEL_BINDING_KEY))
            .ok_or(ServerScramErrorKind::CannotParseClientFinalMessage)?;
        let channel_binding = base64::decode(channel_binding)?;
        if channel_binding != GS2_HEADER.as_bytes() {
            return Err(ServerScramErrorKind::InvalidChannelBinding);
        }

        let nonce = parts
            .get(1)
            .and_then(|s| s.strip_prefix(NONCE_KEY))
            .ok_or(ServerScramErrorKind::CannotParseClientFinalMessage)?;
        if nonce.as_bytes() != client_server_nonce {
            return Err(ServerScramErrorKind::IncorrectClientFinalNonce)?;
        }

        let client_proof = parts
            .last()
            .and_then(|s| s.strip_prefix(PROOF_KEY))
            .ok_or(ServerScramErrorKind::ProofNotFoundInClientFinal)?;

        let without_proof_message_len =
            client_final.len() - (client_proof.len() + PROOF_KEY.len() + 1);
        let client_final_message_without_proof =
            &client_final[0..without_proof_message_len].as_bytes();

        let auth_message = auth_message(
            client_first_message_bare,
            server_first_message,
            client_final_message_without_proof,
        );

        // Verify client proof
        // ClientProof := ClientKey XOR ClientSignature
        // ClientSignature := HMAC(StoredKey, AuthMessage)
        // Inverse of XOR is XOR
        let client_signature = self.hmac(stored_password.stored_key, &auth_message)?;
        let client_proof = base64::decode(client_proof)?;
        let client_key = xor(&client_proof, &client_signature)?;
        // StoredKey := H(ClientKey)
        let stored_key_from_client = self.h(&client_key);
        if stored_key_from_client != stored_password.stored_key {
            return Err(ServerScramErrorKind::AuthenticationFailed);
        }

        let server_signature_bytes = self.hmac(stored_password.server_key, &auth_message)?;
        let server_signature = base64::encode(server_signature_bytes);

        // Form server final message
        let mut server_final = Vec::new();
        server_final.put_slice(VERIFIER_KEY.as_bytes());
        server_final.put_slice(server_signature.as_bytes());
        Ok(server_final)
    }
}

#[cfg(feature = "acceptor")]
pub(crate) struct ServerFirstMessage<'a> {
    pub username: &'a str,
    pub client_first_message_bare: Bytes,
    pub client_server_nonce: Bytes,
    pub message: Bytes,
}

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

/// SCRAM authenticator
#[derive(Debug, Clone)]
pub struct ScramAuthenticator<C: ScramCredentialProvider + Clone> {
    credentials: C,
    state: ScramAuthenticatorState,
}

impl<C> ScramAuthenticator<C>
where
    C: ScramCredentialProvider + Clone,
{
    /// Creates a new SCRAM authenticator
    pub fn new(credentials: C) -> Self {
        Self {
            credentials,
            state: ScramAuthenticatorState::Initial,
        }
    }

    /// Get the underlying credentials
    pub fn credentials(&self) -> &C {
        &self.credentials
    }

    pub(crate) fn compute_server_first_message(
        &mut self,
        client_first_message: &[u8],
    ) -> Result<Option<impl Into<Vec<u8>>>, ServerScramErrorKind> {
        use base64::Engine;

        let nonce = generate_nonce();
        let base64_server_nonce = base64::engine::general_purpose::STANDARD.encode(nonce);

        let server_first = match self
            .credentials
            .scram_version()
            .compute_server_first_message(
                client_first_message,
                &base64_server_nonce,
                &self.credentials,
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

        Ok(Some(server_first.message.to_vec()))
    }

    pub(crate) fn compute_server_final_message(
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

                let server_final_message = self
                    .credentials
                    .scram_version()
                    .compute_server_final_message(
                        client_final_message,
                        client_server_nonce,
                        client_first_message_bare,
                        server_first_message,
                        &stored_password,
                    )?;
                self.state = ScramAuthenticatorState::ServerFinalSent;
                Ok(Some(server_final_message))
            }
            _ => Err(ServerScramErrorKind::IllegalAuthenticatorState),
        }
    }
}
