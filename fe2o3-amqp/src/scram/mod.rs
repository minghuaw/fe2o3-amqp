//! Implements SCRAM for SASL-SCRAM-SHA1 and SASL-SCRAM-SHA256 auth

use std::ops::BitXor;

use bytes::{BufMut, Bytes, BytesMut};
use hmac::{
    digest::{Digest, FixedOutput, KeyInit},
    Hmac, Mac,
};
use rand::Rng;
use sha1::Sha1;
use sha2::Sha256;

use self::{
    attributes::{
        CHANNEL_BINDING_KEY, GS2_HEADER, ITERATION_COUNT_KEY, NONCE_KEY, PROOF_KEY, RESERVED_MEXT,
        SALT_KEY, USERNAME_KEY, VERIFIER_KEY,
    },
    error::ScramErrorKind,
};

mod attributes;
pub(crate) mod error;

#[derive(Debug, Clone)]
enum ScramClientState {
    Initial,
    ClientFirstSent {
        client_nonce: String,
        client_first: Vec<u8>,
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

    pub fn compute_client_first(&mut self) -> Vec<u8> {
        let nonce_bytes: [u8; 32] = rand::thread_rng().gen();
        let nonce = base64::encode(nonce_bytes);
        let client_first = self
            .scram
            .client_first(self.username.as_bytes(), nonce.as_bytes());
        self.state = ScramClientState::ClientFirstSent {
            client_nonce: nonce,
            client_first: client_first.clone(),
        };
        client_first.into()
    }

    pub fn compute_client_final(&mut self, server_first: &str) -> Result<Vec<u8>, ScramErrorKind> {
        match &self.state {
            ScramClientState::ClientFirstSent {
                client_nonce,
                client_first,
            } => {
                let (client_final, server_signature) = self
                    .scram
                    .compute_client_final(&client_nonce, &self.password, server_first, &client_first)?;
                self.state = ScramClientState::ClientFinalSent { server_signature };
                Ok(client_final)
            },
            _ => Err(ScramErrorKind::IllegalClientState),
        }
    }

    pub fn validate_server_final(&mut self, server_final: &[u8]) -> Result<(), ScramErrorKind> {
        match &self.state {
            ScramClientState::ClientFinalSent { server_signature } => {
                self.scram.validate_server_final(server_final, server_signature)?;
                self.state = ScramClientState::Complete;
                Ok(())
            },
            _ => Err(ScramErrorKind::IllegalClientState)
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ScramVersion {
    Sha1,
    Sha256,
}

// This is shamelessly copied from
// [MongoDB](https://github.com/mongodb/mongo-rust-driver/blob/main/src/client/auth/scram.rs)
impl ScramVersion {
    pub(crate) fn client_first(&self, username: &[u8], nonce: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.put_slice(GS2_HEADER.as_bytes());

        bytes.put_slice(USERNAME_KEY.as_bytes());
        bytes.put_slice(username);

        bytes.put_u8(',' as u8);

        bytes.put_slice(NONCE_KEY.as_bytes());
        bytes.put_slice(nonce);

        bytes
    }

    fn h_i(&self, password: &[u8], salt: &[u8], iterations: u32) -> Vec<u8> {
        match self {
            ScramVersion::Sha256 => h_i::<Hmac<Sha1>>(password, salt, iterations, 160 / 8),
            ScramVersion::Sha1 => h_i::<Hmac<Sha256>>(password, salt, iterations, 256 / 8),
        }
    }

    fn compute_salted_password(
        &self,
        password: &str,
        salt: &[u8],
        iterations: u32,
    ) -> Result<Vec<u8>, ScramErrorKind> {
        let normalized_password = stringprep::saslprep(password)?;
        Ok(self.h_i(normalized_password.as_bytes(), salt, iterations))
    }

    /// HMAC function used as part of SCRAM authentication.
    fn hmac(&self, key: &[u8], input: &[u8]) -> Result<Vec<u8>, ScramErrorKind> {
        let bytes = match self {
            ScramVersion::Sha1 => mac::<Hmac<Sha1>>(key, input, "SCRAM")?.as_ref().into(),
            ScramVersion::Sha256 => mac::<Hmac<Sha256>>(key, input, "SCRAM")?.as_ref().into(),
        };

        Ok(bytes)
    }

    /// The "h" function defined in the SCRAM RFC.
    ///
    /// H(str): Apply the cryptographic hash function to the octet string "str", producing an octet
    /// string as a result. The size of the result depends on the hash result size for the hash
    /// function in use.
    fn h(&self, str: &[u8]) -> Vec<u8> {
        match self {
            ScramVersion::Sha1 => hash::<Sha1>(str),
            ScramVersion::Sha256 => hash::<Sha256>(str),
        }
    }

    pub(crate) fn compute_client_final(
        &self,
        client_nonce: &str,
        password: &str,
        server_first: &str,
        client_first: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), ScramErrorKind> {
        let parts: Vec<&str> = server_first.split(',').collect();

        if parts.len() < 3 {
            return Err(ScramErrorKind::InsufficientParts);
        } else if parts[0].starts_with(RESERVED_MEXT) {
            return Err(ScramErrorKind::ExtensionNotSupported);
        }

        let server_nonce = parts[0]
            .strip_prefix(NONCE_KEY)
            .ok_or(ScramErrorKind::NonceNotFound)?;
        if !server_nonce.starts_with(client_nonce) {
            return Err(ScramErrorKind::ClientNonceMismatch);
        }

        let base64_salt = parts[1]
            .strip_prefix(SALT_KEY)
            .ok_or(ScramErrorKind::SaltNotFound)?;
        let salt = base64::decode(base64_salt)?;

        let iter_count_str = parts[2]
            .strip_prefix(ITERATION_COUNT_KEY)
            .ok_or(ScramErrorKind::IterationCountNotFound)?;
        let iterations: u32 = iter_count_str
            .parse()
            .map_err(|_| ScramErrorKind::IterationCountParseError)?;

        // SaltedPassword := Hi(Normalize(password), salt, i)
        // ClientKey := HMAC(SaltedPassword, "Client Key")
        // StoredKey := H(ClientKey)
        // AuthMessage := client-first-message-bare + "," +
        //             server-first-message + "," +
        //             client-final-message-without-proof
        // ClientSignature := HMAC(StoredKey, AuthMessage)
        // ClientProof := ClientKey XOR ClientSignature
        // ServerKey := HMAC(SaltedPassword, "Server Key")
        // ServerSignature := HMAC(ServerKey, AuthMessage)

        let salted_password = self.compute_salted_password(password, &salt[..], iterations)?;
        let client_key = self.hmac(&salted_password, b"Client Key")?;
        let stored_key = self.h(&client_key);
        let client_final_message_without_proof = without_proof(server_nonce);
        let auth_message = auth_message(
            client_first,
            server_first.as_bytes(),
            &client_final_message_without_proof,
        );

        let client_signature = self.hmac(&stored_key, &auth_message)?;
        let client_proof = base64::encode(xor(&client_key, &client_signature)?);

        let client_final =
            client_final(&client_final_message_without_proof, client_proof.as_bytes());

        let server_key = self.hmac(&salted_password, b"Server Key")?;
        let server_signature = self.hmac(&server_key, &auth_message)?;

        Ok((client_final, server_signature))
    }
    
    fn validate_server_final(&self, server_final: &[u8], server_signature: &[u8]) -> Result<(), ScramErrorKind> {
        let server_final = std::str::from_utf8(server_final)?;
        let parts: Vec<&str> = server_final.split(',').collect();
        
        let signature = parts.get(0)
            .and_then(|signature| 
                signature.strip_prefix(VERIFIER_KEY)
            )
            .ok_or(ScramErrorKind::ServerSignatureMismatch)?;
        let signature_bytes = base64::decode(signature)?;
        
        match signature_bytes == server_signature {
            true => Ok(()),
            false => Err(ScramErrorKind::ServerSignatureMismatch),
        }
    }
}

fn client_final(client_final_message_without_proof: &[u8], client_proof: &[u8]) -> Vec<u8> {
    let total_len =
        client_final_message_without_proof.len() + 1 + PROOF_KEY.len() + client_proof.len();
    let mut buf = Vec::with_capacity(total_len);
    buf.put_slice(client_final_message_without_proof);
    buf.put_u8(b',');
    buf.put_slice(client_proof);
    buf
}

fn auth_message(
    client_first_message_bare: &[u8],
    server_first_message: &[u8],
    client_final_message_without_proof: &[u8],
) -> Vec<u8> {
    let total_len = client_first_message_bare.len()
        + 1
        + server_first_message.len()
        + 1
        + client_final_message_without_proof.len();
    let mut buf = Vec::with_capacity(total_len);

    buf.put_slice(client_first_message_bare);
    buf.put_u8(b',');
    buf.put_slice(server_first_message);
    buf.put_u8(b',');
    buf.put_slice(client_final_message_without_proof);
    buf
}

fn without_proof(server_nonce: &str) -> Vec<u8> {
    let encoded_gs2_header = base64::encode(GS2_HEADER).into_bytes();
    let total_len = CHANNEL_BINDING_KEY.len()
        + encoded_gs2_header.len()
        + 1
        + NONCE_KEY.len()
        + server_nonce.len();
    let mut buf = Vec::with_capacity(total_len);

    buf.put_slice(CHANNEL_BINDING_KEY.as_bytes());
    buf.put_slice(&encoded_gs2_header);

    buf.put_u8(b',');

    buf.put_slice(NONCE_KEY.as_bytes());
    buf.put_slice(server_nonce.as_bytes());

    buf
}

// Hi() is, essentially, PBKDF2 [RFC2898] with HMAC() as the pseudorandom function (PRF) and with
// dkLen == output length of HMAC() == output length of H().
//
// This is (shamelessly) copied from MongoDB driver
fn h_i<M: KeyInit + FixedOutput + Mac + Sync + Clone>(
    password: &[u8],
    salt: &[u8],
    iterations: u32,
    output_size: usize,
) -> Vec<u8> {
    let mut buf = vec![0u8; output_size];
    pbkdf2::pbkdf2::<M>(password, salt, iterations, buf.as_mut_slice());
    buf
}

fn mac<M: Mac + KeyInit>(
    key: &[u8],
    input: &[u8],
    auth_mechanism: &str,
) -> Result<impl AsRef<[u8]>, ScramErrorKind> {
    let mut mac =
        <M as Mac>::new_from_slice(key).map_err(|_| ScramErrorKind::HmacErrorInvalidLength)?;
    mac.update(input);
    Ok(mac.finalize().into_bytes())
}

fn hash<D: Digest>(val: &[u8]) -> Vec<u8> {
    let mut hash = D::new();
    hash.update(val);
    hash.finalize().to_vec()
}

fn xor(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, ScramErrorKind> {
    if lhs.len() != rhs.len() {
        return Err(ScramErrorKind::XorLengthMismatch);
    }

    Ok(lhs
        .iter()
        .zip(rhs.iter())
        .map(|(l, r)| l.bitxor(r))
        .collect())
}
