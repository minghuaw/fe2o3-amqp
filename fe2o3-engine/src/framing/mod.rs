//! Implements low level transport framing
//!
//! Idea: Two layer design.
//! layer 0: `tokio_util::codec::LengthDelimited` over `AsyncWrite`
//! layer 1: A custom encoder that implements `tokio_util::codec::Encoder` and takes a Frame as an item
//!
//! Layer 0 should be hidden within the connection and there should be API that provide
//! access to layer 1 for types that implement Encoder

pub const FRAME_TYPE_AMQP: u8 = 0x00;
pub const FRAME_TYPE_SASL: u8 = 0x01;

pub mod amqp;
pub mod protocol_header;
pub mod transport;
