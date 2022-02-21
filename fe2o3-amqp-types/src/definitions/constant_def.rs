
/// the IANA assigned port number for AMQP.
/// The standard AMQP port number that has been assigned
/// by IANA for TCP, UDP, and SCTP.
/// There are currently no UDP or SCTP mappings defined
/// for AMQP. The port number is reserved for future transport
/// mappings to these protocols
pub const PORT: u16 = 5672;

/// the IANA assigned port number for secure AMQP (amqps).
/// The standard AMQP port number that has been assigned
/// by IANA for secure TCP using TLS.
/// Implementations listening on this port SHOULD NOT expect a protocol handshake before TLS is negotiated.
pub const SECURE_PORT: u16 = 5671;

/// major protocol version.
pub const MAJOR: u8 = 1;

///  minor protocol version.
pub const MINOR: u8 = 0;

/// protocol revision
pub const REVISION: u8 = 0;

/// the lower bound for the agreed maximum frame size (in
/// bytes).
/// During the initial connection negotiation, the two peers
/// MUST agree upon a maximum frame size. This constant
/// defines the minimum value to which the maximum frame
/// size can be set. By defining this value, the peers can guarantee that they can send frames of up to this size until they
/// have agreed a definitive maximum frame size for that connection.
pub const MIN_MAX_FRAME_SIZE: usize = 512;
