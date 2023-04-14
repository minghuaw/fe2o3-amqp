//! SCRAM

use std::{ops::BitXor, sync::Arc};

use bytes::{BufMut, Bytes, BytesMut};
use hmac::{
    digest::{Digest, FixedOutput, InvalidLength, KeyInit},
    Hmac, Mac,
};
use rand::Rng;
use sha1::Sha1;
use sha2::{Sha256, Sha512};

use attributes::{
    CHANNEL_BINDING_KEY, GS2_HEADER, ITERATION_COUNT_KEY, NONCE_KEY, PROOF_KEY, RESERVED_MEXT,
    SALT_KEY, USERNAME_KEY, VERIFIER_KEY,
};
use error::XorLengthMismatch;

mod attributes;
pub(crate) mod client;
mod error;
pub use error::*;

#[cfg_attr(docsrs, doc(cfg(feature = "acceptor")))]
#[cfg(feature = "acceptor")]
mod server;

#[cfg(feature = "acceptor")]
pub use server::ScramAuthenticator;

use crate::sasl_profile::{SCRAM_SHA_1, SCRAM_SHA_256, SCRAM_SHA_512};

/// Default iterations for SCRAM
pub const DEFAULT_SCRAM_ITERATIONS: u32 = 4096;

/// The server stores only the `username`, `salt`, `iteration-count`, `StoredKey`, `ServerKey`.
///
/// `StoredKey` = `H(ClientKey)`
#[derive(Debug)]
pub struct StoredPassword<'a> {
    /// Salt
    pub salt: &'a [u8],

    /// Iterators
    pub iterations: u32,

    /// StoredKey := H(ClientKey)
    pub stored_key: &'a [u8],

    /// ServerKey := HMAC(SaltedPassword, "Server Key")
    pub server_key: &'a [u8],
}

/// Provide credential for SCRAM credentials
pub trait ScramCredentialProvider {
    /// Get the SCRAM version for this credential
    fn scram_version(&self) -> &ScramVersion;

    /// Get the stored password
    fn get_stored_password<'a>(&'a self, username: &str) -> Option<StoredPassword<'a>>;
}

impl<T> ScramCredentialProvider for Arc<T>
where
    T: ScramCredentialProvider,
{
    fn scram_version(&self) -> &ScramVersion {
        self.as_ref().scram_version()
    }

    fn get_stored_password<'a>(&'a self, username: &str) -> Option<StoredPassword<'a>> {
        self.as_ref().get_stored_password(username)
    }
}

/// SCRAM version
#[derive(Debug, Clone)]
pub enum ScramVersion {
    /// SHA-1
    Sha1,

    /// SHA-256
    Sha256,

    /// SHA-512
    Sha512,
}

// This is shamelessly copied from
// [MongoDB](https://github.com/mongodb/mongo-rust-driver/blob/main/src/client/auth/scram.rs)
impl ScramVersion {
    /// Get the mechanism string
    pub fn mechanism(&self) -> &'static str {
        match self {
            ScramVersion::Sha1 => SCRAM_SHA_1,
            ScramVersion::Sha256 => SCRAM_SHA_256,
            ScramVersion::Sha512 => SCRAM_SHA_512,
        }
    }

    /// Returns (client_first_message, client_first_message_bare)
    pub(crate) fn client_first_message(&self, username: &[u8], nonce: &[u8]) -> (Bytes, Bytes) {
        let mut bytes = BytesMut::new();
        bytes.put_slice(GS2_HEADER.as_bytes());

        bytes.put_slice(USERNAME_KEY.as_bytes());
        bytes.put_slice(username);

        bytes.put_u8(b',');

        bytes.put_slice(NONCE_KEY.as_bytes());
        bytes.put_slice(nonce);

        let client_first_message = bytes.freeze();
        let gs2_header_len = GS2_HEADER.as_bytes().len();
        let client_first_message_bare = client_first_message.slice(gs2_header_len..);
        (client_first_message, client_first_message_bare)
    }

    pub(crate) fn h_i(&self, password: &[u8], salt: &[u8], iterations: u32) -> Vec<u8> {
        match self {
            ScramVersion::Sha1 => h_i::<Hmac<Sha1>>(password, salt, iterations, 160 / 8),
            ScramVersion::Sha256 => h_i::<Hmac<Sha256>>(password, salt, iterations, 256 / 8),
            ScramVersion::Sha512 => h_i::<Hmac<Sha512>>(password, salt, iterations, 512 / 8),
        }
    }

    pub(crate) fn compute_salted_password(
        &self,
        password: &str,
        salt: &[u8],
        iterations: u32,
    ) -> Result<Vec<u8>, stringprep::Error> {
        let normalized_password = stringprep::saslprep(password)?;
        Ok(self.h_i(normalized_password.as_bytes(), salt, iterations))
    }

    /// HMAC function used as part of SCRAM authentication.
    pub(crate) fn hmac(&self, key: &[u8], input: &[u8]) -> Result<Vec<u8>, InvalidLength> {
        let bytes = match self {
            ScramVersion::Sha1 => mac::<Hmac<Sha1>>(key, input)?.as_ref().into(),
            ScramVersion::Sha256 => mac::<Hmac<Sha256>>(key, input)?.as_ref().into(),
            ScramVersion::Sha512 => mac::<Hmac<Sha512>>(key, input)?.as_ref().into(),
        };

        Ok(bytes)
    }

    /// The "h" function defined in the SCRAM RFC.
    ///
    /// H(str): Apply the cryptographic hash function to the octet string "str", producing an octet
    /// string as a result. The size of the result depends on the hash result size for the hash
    /// function in use.
    ///
    /// H(str): Apply the cryptographic hash function to the octet string
    /// "str", producing an octet string as a result. The size of the
    /// result depends on the hash result size for the hash function in use
    ///
    ///  dkLen == output length of HMAC() == output length of H()
    pub(crate) fn h(&self, str: &[u8]) -> Vec<u8> {
        match self {
            ScramVersion::Sha1 => hash::<Sha1>(str),
            ScramVersion::Sha256 => hash::<Sha256>(str),
            ScramVersion::Sha512 => hash::<Sha512>(str),
        }
    }

    pub(crate) fn compute_client_final_message(
        &self,
        client_nonce: &str,
        password: &str,
        server_first: &str,
        client_first_message_bare: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), ScramErrorKind> {
        use base64::Engine;

        let parts: Vec<&str> = server_first.split(',').collect();

        if parts.len() < 3 {
            return Err(ScramErrorKind::InsufficientParts);
        } else if parts[0].starts_with(RESERVED_MEXT) {
            return Err(ScramErrorKind::ExtensionNotSupported);
        }

        let client_server_nonce = parts[0]
            .strip_prefix(NONCE_KEY)
            .ok_or(ScramErrorKind::NonceNotFound)?;
        if !client_server_nonce.starts_with(client_nonce) {
            return Err(ScramErrorKind::ClientNonceMismatch);
        }

        let base64_salt = parts[1]
            .strip_prefix(SALT_KEY)
            .ok_or(ScramErrorKind::SaltNotFound)?;
        let salt = base64::engine::general_purpose::STANDARD.decode(base64_salt)?;

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
        let client_final_message_without_proof = without_proof(client_server_nonce);
        let auth_message = auth_message(
            client_first_message_bare,
            server_first.as_bytes(),
            &client_final_message_without_proof,
        );

        let client_proof_bytes =
            self.compute_client_proof::<ScramErrorKind>(&salted_password, &auth_message)?;
        let client_proof = base64::engine::general_purpose::STANDARD.encode(client_proof_bytes);

        let client_final =
            client_final(&client_final_message_without_proof, client_proof.as_bytes());

        let server_signature =
            self.compute_server_signature::<ScramErrorKind>(&salted_password, &auth_message)?;

        Ok((client_final, server_signature))
    }

    fn validate_server_final(
        &self,
        server_final: &[u8],
        server_signature: &[u8],
    ) -> Result<(), ScramErrorKind> {
        use base64::Engine;

        let server_final = std::str::from_utf8(server_final)?;
        let parts: Vec<&str> = server_final.split(',').collect();

        let signature = parts
            .first()
            .and_then(|signature| signature.strip_prefix(VERIFIER_KEY))
            .ok_or(ScramErrorKind::ServerSignatureMismatch)?;
        let signature_bytes = base64::engine::general_purpose::STANDARD.decode(signature)?;

        match signature_bytes == server_signature {
            true => Ok(()),
            false => Err(ScramErrorKind::ServerSignatureMismatch),
        }
    }

    #[cfg(feature = "acceptor")]
    fn compute_server_first_message<'a, C>(
        &self,
        client_first: &'a [u8],
        base64_server_nonce: &str,
        credentials: &C,
    ) -> Result<Option<ServerFirstMessage<'a>>, ServerScramErrorKind>
    where
        C: ScramCredentialProvider,
    {
        use base64::Engine;

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
        let base64_salt = base64::engine::general_purpose::STANDARD.encode(stored_password.salt);
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

    fn compute_server_signature<E>(
        &self,
        salted_password: &[u8],
        auth_message: &[u8],
    ) -> Result<Vec<u8>, E>
    where
        E: From<stringprep::Error> + From<InvalidLength>,
    {
        let server_key = self.hmac(salted_password, b"Server Key")?;
        let server_signature = self.hmac(&server_key, auth_message)?;
        Ok(server_signature)
    }

    #[cfg(feature = "acceptor")]
    fn compute_server_final_message(
        &self,
        client_final: &[u8],
        client_server_nonce: &[u8],
        client_first_message_bare: &[u8],
        server_first_message: &[u8],
        stored_password: &StoredPassword,
    ) -> Result<Vec<u8>, ServerScramErrorKind> {
        use base64::Engine;

        let client_final = std::str::from_utf8(client_final)?;
        let parts: Vec<&str> = client_final.split(',').collect();

        let channel_binding = parts
            .first()
            .and_then(|s| s.strip_prefix(CHANNEL_BINDING_KEY))
            .ok_or(ServerScramErrorKind::CannotParseClientFinalMessage)?;
        let channel_binding = base64::engine::general_purpose::STANDARD.decode(channel_binding)?;
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
        let client_proof = base64::engine::general_purpose::STANDARD.decode(client_proof)?;
        let client_key = xor(&client_proof, &client_signature)?;
        // StoredKey := H(ClientKey)
        let stored_key_from_client = self.h(&client_key);
        if stored_key_from_client != stored_password.stored_key {
            return Err(ServerScramErrorKind::AuthenticationFailed);
        }

        let server_signature_bytes = self.hmac(stored_password.server_key, &auth_message)?;
        let server_signature =
            base64::engine::general_purpose::STANDARD.encode(server_signature_bytes);

        // Form server final message
        let mut server_final = Vec::new();
        server_final.put_slice(VERIFIER_KEY.as_bytes());
        server_final.put_slice(server_signature.as_bytes());
        Ok(server_final)
    }

    fn compute_client_proof<E>(
        &self,
        salted_password: &[u8],
        auth_message: &[u8],
    ) -> Result<Vec<u8>, E>
    where
        E: From<InvalidLength> + From<XorLengthMismatch>,
    {
        let client_key = self.hmac(salted_password, b"Client Key")?;
        let stored_key = self.h(&client_key);
        let client_signature = self.hmac(&stored_key, auth_message)?;
        xor(&client_key, &client_signature).map_err(Into::into)
    }
}

#[cfg(feature = "acceptor")]
pub(crate) struct ServerFirstMessage<'a> {
    pub username: &'a str,
    pub client_first_message_bare: Bytes,
    pub client_server_nonce: Bytes,
    pub message: Bytes,
}

fn client_final(client_final_message_without_proof: &[u8], client_proof: &[u8]) -> Vec<u8> {
    let total_len =
        client_final_message_without_proof.len() + 1 + PROOF_KEY.len() + client_proof.len();
    let mut buf = Vec::with_capacity(total_len);
    buf.put_slice(client_final_message_without_proof);
    buf.put_u8(b',');
    buf.put_slice(PROOF_KEY.as_bytes());
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

fn without_proof(client_server_nonce: &str) -> Vec<u8> {
    use base64::Engine;

    let encoded_gs2_header = base64::engine::general_purpose::STANDARD
        .encode(GS2_HEADER)
        .into_bytes();
    let total_len = CHANNEL_BINDING_KEY.len()
        + encoded_gs2_header.len()
        + 1
        + NONCE_KEY.len()
        + client_server_nonce.len();
    let mut buf = Vec::with_capacity(total_len);

    buf.put_slice(CHANNEL_BINDING_KEY.as_bytes());
    buf.put_slice(&encoded_gs2_header);

    buf.put_u8(b',');

    buf.put_slice(NONCE_KEY.as_bytes());
    buf.put_slice(client_server_nonce.as_bytes());

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

fn mac<M: Mac + KeyInit>(key: &[u8], input: &[u8]) -> Result<impl AsRef<[u8]>, InvalidLength> {
    let mut mac = <M as Mac>::new_from_slice(key)?;
    mac.update(input);
    Ok(mac.finalize().into_bytes())
}

fn hash<D: Digest>(val: &[u8]) -> Vec<u8> {
    let mut hash = D::new();
    hash.update(val);
    hash.finalize().to_vec()
}

fn xor(lhs: &[u8], rhs: &[u8]) -> Result<Vec<u8>, XorLengthMismatch> {
    if lhs.len() != rhs.len() {
        return Err(XorLengthMismatch {});
    }

    Ok(lhs
        .iter()
        .zip(rhs.iter())
        .map(|(l, r)| l.bitxor(r))
        .collect())
}

fn generate_nonce() -> [u8; 32] {
    rand::thread_rng().gen()
}

#[cfg(test)]
#[allow(unused)]
mod tests {
    use super::attributes::{GS2_HEADER, NONCE_KEY, SALT_KEY};
    use super::{ScramCredentialProvider, ScramVersion, StoredPassword};

    struct TestScramCredential {
        scram_version: ScramVersion,
        username: String,
        salt: Vec<u8>,
        iterations: u32,
        stored_key: Vec<u8>,
        server_key: Vec<u8>,
    }

    impl TestScramCredential {
        pub fn new(
            scram_version: ScramVersion,
            username: &str,
            password: &str,
            salt: Vec<u8>,
            iterations: u32,
        ) -> Self {
            let salted_password = scram_version
                .compute_salted_password(password, &salt, iterations)
                .unwrap();
            let client_key = scram_version.hmac(&salted_password, b"Client Key").unwrap();
            let stored_key = scram_version.h(&client_key);
            let server_key = scram_version.hmac(&salted_password, b"Server Key").unwrap();
            Self {
                scram_version,
                salt,
                username: username.into(),
                iterations,
                stored_key,
                server_key,
            }
        }
    }

    impl ScramCredentialProvider for TestScramCredential {
        fn scram_version(&self) -> &ScramVersion {
            &self.scram_version
        }

        fn get_stored_password(&self, username: &str) -> Option<StoredPassword> {
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

    mod scram_sha1 {
        use super::super::ScramVersion;

        pub(super) static ITERATIONS: u32 = 4096;
        pub(super) static TEST_USERNAME: &str = "user";
        pub(super) static TEST_PASSWORD: &str = "pencil";
        pub(super) static CLIENT_NONCE: &str = "fyko+d2lbbFgONRv9qkxdawL";
        pub(super) static EXPECTED_CLIENT_INITIAL_RESPONSE: &str =
            "n,,n=user,r=fyko+d2lbbFgONRv9qkxdawL";
        pub(super) static SERVER_FIRST_MESSAGE: &str =
            "r=fyko+d2lbbFgONRv9qkxdawL3rfcNHYJY1ZVvWVs7j,s=QSXCR+Q6sek8bf92,i=4096";
        pub(super) static EXPECTED_CLIENT_FINAL_MESSAGE: &str =
            "c=biws,r=fyko+d2lbbFgONRv9qkxdawL3rfcNHYJY1ZVvWVs7j,p=v0X8v3Bz2T0CJGbJQyF0X+HI4Ts=";
        pub(super) static SERVER_FINAL_MESSAGE: &str = "v=rmF9pqV8S7suAoZWja4dJRkFsKQ=";
        pub(super) static VERSION: ScramVersion = ScramVersion::Sha1;
    }

    mod scram_sha256 {
        use super::super::ScramVersion;

        pub(super) static ITERATIONS: u32 = 4096;
        pub(super) static TEST_USERNAME: &str = "user";
        pub(super) static TEST_PASSWORD: &str = "pencil";
        pub(super) static CLIENT_NONCE: &str = "rOprNGfwEbeRWgbNEkqO";
        pub(super) static EXPECTED_CLIENT_INITIAL_RESPONSE: &str =
            "n,,n=user,r=rOprNGfwEbeRWgbNEkqO";
        pub(super) static SERVER_FIRST_MESSAGE: &str =
            "r=rOprNGfwEbeRWgbNEkqO%hvYDpWUa2RaTCAfuxFIlj)hNlF$k0,s=W22ZaJ0SNY7soEsUEjb6gQ==,i=4096";
        pub(super) static EXPECTED_CLIENT_FINAL_MESSAGE: &str = "c=biws,r=rOprNGfwEbeRWgbNEkqO%hvYDpWUa2RaTCAfuxFIlj)hNlF$k0,p=dHzbZapWIk4jUhN+Ute9ytag9zjfMHgsqmmiz7AndVQ=";
        pub(super) static SERVER_FINAL_MESSAGE: &str =
            "v=6rriTRBi23WpRR/wtup+mMhUZUn/dB5nLTJRsjl95G4=";
        pub(super) static VERSION: ScramVersion = ScramVersion::Sha256;
    }

    mod scram_sha512 {
        use super::super::ScramVersion;

        pub(super) static ITERATIONS: u32 = 4096;
        pub(super) static TEST_USERNAME: &str = "user";
        pub(super) static TEST_PASSWORD: &str = "pencil";
        pub(super) static CLIENT_NONCE: &str = "rOprNGfwEbeRWgbNEkqO";
        pub(super) static EXPECTED_CLIENT_INITIAL_RESPONSE: &str =
            "n,,n=user,r=rOprNGfwEbeRWgbNEkqO";
        pub(super) static SERVER_FIRST_MESSAGE: &str = "r=rOprNGfwEbeRWgbNEkqO02431b08-2f89-4bad-a4e6-80c0564ec865,s=Yin2FuHTt/M0kJWb0t9OI32n2VmOGi3m+JfjOvuDF88=,i=4096";
        pub(super) static EXPECTED_CLIENT_FINAL_MESSAGE: &str = "c=biws,r=rOprNGfwEbeRWgbNEkqO02431b08-2f89-4bad-a4e6-80c0564ec865,p=Hc5yec3NmCD7t+kFRw4/3yD6/F3SQHc7AVYschRja+Bc3sbdjlA0eH1OjJc0DD4ghn1tnXN5/Wr6qm9xmaHt4A==";
        pub(super) static SERVER_FINAL_MESSAGE: &str = "v=BQuhnKHqYDwQWS5jAw4sZed+C9KFUALsbrq81bB0mh+bcUUbbMPNNmBIupnS2AmyyDnG5CTBQtkjJ9kyY4kzmw==";
        pub(super) static VERSION: ScramVersion = ScramVersion::Sha512;
    }

    fn get_base64_server_nonce_from_server_first_message<'a>(
        server_first_message: &'a str,
        client_nonce: &'a str,
    ) -> &'a str {
        let parts: Vec<&str> = server_first_message.split(',').collect();
        let both_nonce = parts[0].strip_prefix(NONCE_KEY).unwrap();
        both_nonce.strip_prefix(client_nonce).unwrap()
    }

    fn get_base64_salt_from_server_first_message(server_first_message: &str) -> &str {
        let parts: Vec<&str> = server_first_message.split(',').collect();
        parts[1].strip_prefix(SALT_KEY).unwrap()
    }

    fn get_client_first_message_bare(client_first_message: &str) -> &str {
        &client_first_message[GS2_HEADER.len()..]
    }

    #[test]
    fn test_sasl_scram_sha1_client() {
        use scram_sha1::*;

        let (client_first_message, client_first_message_bare) =
            VERSION.client_first_message(TEST_USERNAME.as_bytes(), CLIENT_NONCE.as_bytes());
        assert_eq!(
            client_first_message,
            EXPECTED_CLIENT_INITIAL_RESPONSE.as_bytes()
        );

        let (client_final, server_signature) = VERSION
            .compute_client_final_message(
                CLIENT_NONCE,
                TEST_PASSWORD,
                SERVER_FIRST_MESSAGE,
                &client_first_message_bare,
            )
            .unwrap();
        assert_eq!(client_final, EXPECTED_CLIENT_FINAL_MESSAGE.as_bytes());
        assert!(VERSION
            .validate_server_final(SERVER_FINAL_MESSAGE.as_bytes(), &server_signature)
            .is_ok());
    }

    #[test]
    fn test_sasl_scram_sha256_client() {
        use scram_sha256::*;

        let (client_first_message, client_first_message_bare) =
            VERSION.client_first_message(TEST_USERNAME.as_bytes(), CLIENT_NONCE.as_bytes());
        assert_eq!(
            client_first_message,
            EXPECTED_CLIENT_INITIAL_RESPONSE.as_bytes()
        );

        let (client_final, server_signature) = VERSION
            .compute_client_final_message(
                CLIENT_NONCE,
                TEST_PASSWORD,
                SERVER_FIRST_MESSAGE,
                &client_first_message_bare,
            )
            .unwrap();
        assert_eq!(client_final, EXPECTED_CLIENT_FINAL_MESSAGE.as_bytes());
        assert!(VERSION
            .validate_server_final(SERVER_FINAL_MESSAGE.as_bytes(), &server_signature)
            .is_ok());
    }

    #[test]
    fn test_sasl_scram_sha512_client() {
        use scram_sha512::*;

        let (client_first_message, client_first_message_bare) =
            VERSION.client_first_message(TEST_USERNAME.as_bytes(), CLIENT_NONCE.as_bytes());
        assert_eq!(
            client_first_message,
            EXPECTED_CLIENT_INITIAL_RESPONSE.as_bytes()
        );

        let (client_final, server_signature) = VERSION
            .compute_client_final_message(
                CLIENT_NONCE,
                TEST_PASSWORD,
                SERVER_FIRST_MESSAGE,
                &client_first_message_bare,
            )
            .unwrap();
        assert_eq!(client_final, EXPECTED_CLIENT_FINAL_MESSAGE.as_bytes());
        assert!(VERSION
            .validate_server_final(SERVER_FINAL_MESSAGE.as_bytes(), &server_signature)
            .is_ok());
    }

    #[cfg(feature = "acceptor")]
    #[test]
    fn test_sasl_scram_sha1_server() {
        use base64::Engine;
        use scram_sha1::*;

        let base64_salt = get_base64_salt_from_server_first_message(SERVER_FIRST_MESSAGE);
        let salt = base64::engine::general_purpose::STANDARD
            .decode(base64_salt)
            .unwrap();
        let base64_server_nonce =
            get_base64_server_nonce_from_server_first_message(SERVER_FIRST_MESSAGE, CLIENT_NONCE);
        let client_server_nonce = format!("{}{}", CLIENT_NONCE, base64_server_nonce);
        let client_first_message_bare =
            get_client_first_message_bare(EXPECTED_CLIENT_INITIAL_RESPONSE);

        let test_credential = TestScramCredential::new(
            VERSION.clone(),
            TEST_USERNAME,
            TEST_PASSWORD,
            salt,
            ITERATIONS,
        );

        let server_first = VERSION
            .compute_server_first_message(
                EXPECTED_CLIENT_INITIAL_RESPONSE.as_bytes(),
                base64_server_nonce,
                &test_credential,
            )
            .unwrap()
            .unwrap();

        assert_eq!(server_first.username, TEST_USERNAME);
        assert_eq!(server_first.message, SERVER_FIRST_MESSAGE.as_bytes());

        let stored_password = test_credential.get_stored_password(TEST_USERNAME).unwrap();
        let server_final = VERSION
            .compute_server_final_message(
                EXPECTED_CLIENT_FINAL_MESSAGE.as_bytes(),
                client_server_nonce.as_bytes(),
                client_first_message_bare.as_bytes(),
                SERVER_FIRST_MESSAGE.as_bytes(),
                &stored_password,
            )
            .unwrap();
        assert_eq!(server_final, SERVER_FINAL_MESSAGE.as_bytes())
    }

    #[cfg(feature = "acceptor")]
    #[test]
    fn test_sasl_scram_sha256_server() {
        use base64::Engine;
        use scram_sha256::*;

        let base64_salt = get_base64_salt_from_server_first_message(SERVER_FIRST_MESSAGE);
        let salt = base64::engine::general_purpose::STANDARD
            .decode(base64_salt)
            .unwrap();
        let base64_server_nonce =
            get_base64_server_nonce_from_server_first_message(SERVER_FIRST_MESSAGE, CLIENT_NONCE);
        let client_server_nonce = format!("{}{}", CLIENT_NONCE, base64_server_nonce);
        let client_first_message_bare =
            get_client_first_message_bare(EXPECTED_CLIENT_INITIAL_RESPONSE);

        let test_credential = TestScramCredential::new(
            VERSION.clone(),
            TEST_USERNAME,
            TEST_PASSWORD,
            salt,
            ITERATIONS,
        );

        let server_first = VERSION
            .compute_server_first_message(
                EXPECTED_CLIENT_INITIAL_RESPONSE.as_bytes(),
                base64_server_nonce,
                &test_credential,
            )
            .unwrap()
            .unwrap();

        assert_eq!(server_first.username, TEST_USERNAME);
        assert_eq!(server_first.message, SERVER_FIRST_MESSAGE.as_bytes());

        let stored_password = test_credential.get_stored_password(TEST_USERNAME).unwrap();
        let server_final = VERSION
            .compute_server_final_message(
                EXPECTED_CLIENT_FINAL_MESSAGE.as_bytes(),
                client_server_nonce.as_bytes(),
                client_first_message_bare.as_bytes(),
                SERVER_FIRST_MESSAGE.as_bytes(),
                &stored_password,
            )
            .unwrap();
        assert_eq!(server_final, SERVER_FINAL_MESSAGE.as_bytes())
    }

    #[cfg(feature = "acceptor")]
    #[test]
    fn test_sasl_scram_sha512_server() {
        use base64::Engine;
        use scram_sha512::*;

        let base64_salt = get_base64_salt_from_server_first_message(SERVER_FIRST_MESSAGE);
        let salt = base64::engine::general_purpose::STANDARD
            .decode(base64_salt)
            .unwrap();
        let base64_server_nonce =
            get_base64_server_nonce_from_server_first_message(SERVER_FIRST_MESSAGE, CLIENT_NONCE);
        let client_server_nonce = format!("{}{}", CLIENT_NONCE, base64_server_nonce);
        let client_first_message_bare =
            get_client_first_message_bare(EXPECTED_CLIENT_INITIAL_RESPONSE);

        let test_credential = TestScramCredential::new(
            VERSION.clone(),
            TEST_USERNAME,
            TEST_PASSWORD,
            salt,
            ITERATIONS,
        );

        let server_first = VERSION
            .compute_server_first_message(
                EXPECTED_CLIENT_INITIAL_RESPONSE.as_bytes(),
                base64_server_nonce,
                &test_credential,
            )
            .unwrap()
            .unwrap();

        assert_eq!(server_first.username, TEST_USERNAME);
        assert_eq!(server_first.message, SERVER_FIRST_MESSAGE.as_bytes());

        let stored_password = test_credential.get_stored_password(TEST_USERNAME).unwrap();
        let server_final = VERSION
            .compute_server_final_message(
                EXPECTED_CLIENT_FINAL_MESSAGE.as_bytes(),
                client_server_nonce.as_bytes(),
                client_first_message_bare.as_bytes(),
                SERVER_FIRST_MESSAGE.as_bytes(),
                &stored_password,
            )
            .unwrap();
        assert_eq!(server_final, SERVER_FINAL_MESSAGE.as_bytes())
    }
}
