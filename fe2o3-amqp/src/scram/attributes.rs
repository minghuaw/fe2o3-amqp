// /// a: This is an optional attribute, and is part of the GS2 [RFC5801] bridge between the GSS-API and SASL.
// pub const GS2: char = 'a';

pub const GS2_HEADER: &str = "n,,";

/// n: This attribute specifies the name of the user whose password is used for authentication
pub const USERNAME_KEY: &str = "n=";

/// m: This attribute is reserved for future extensibility
pub const RESERVED_MEXT: &str = "m=";

/// r: This attribute specifies a sequence of random printable ASCII characters excluding ',' (which
/// forms the nonce used as input to the hash function).
pub const NONCE_KEY: &str = "r=";

/// c: This REQUIRED attribute specifies the base64-encoded GS2 header and channel binding data.
pub const CHANNEL_BINDING_KEY: &str = "c=";

/// s: This attribute specifies the base64-encoded salt used by the server for the user
pub const SALT_KEY: &str = "s=";

/// i: This attribute specifies an iteration count for the selected hash function and user, and MUST
/// be sent by the server along with the user's salt.
pub const ITERATION_COUNT_KEY: &str = "i=";

/// p: This attribute specifies a base64-encoded ClientProof
pub const PROOF_KEY: &str = "p=";

/// v: This attribute specifies a base64-encoded ServerSignature.
pub const VERIFIER_KEY: &str = "v=";

/// e: This attribute specifies an error that occurred during authentication exchange.
pub const SERVER_ERROR_KEY: &str = "e=";
