//! Implements frame encoder and decoder

pub mod amqp;
pub mod sasl;

/// Type byte of AMQP frame
pub const FRAME_TYPE_AMQP: u8 = 0x00;

/// Type byte of SASL frame
pub const FRAME_TYPE_SASL: u8 = 0x01;

mod error;
pub use error::Error;
