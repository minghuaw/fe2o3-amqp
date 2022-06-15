//! Implements the protocol headers

use std::{convert::{TryFrom, TryInto}, io};

use bytes::{BufMut, Buf, Bytes};
use tokio_util::codec::{Encoder, Decoder};

use super::{error::NegotiationError};

const PROTOCOL_HEADER_PREFIX: &[u8; 4] = b"AMQP";

/// Protocol header
#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolHeader {
    /// Protocol ID
    pub id: ProtocolId,

    /// Major number
    pub major: u8,

    /// Minor number
    pub minor: u8,

    /// Revision number
    pub revision: u8,
}

impl Default for ProtocolHeader {
    fn default() -> Self {
        Self {
            id: ProtocolId::Amqp,
            major: fe2o3_amqp_types::definitions::MAJOR,
            minor: fe2o3_amqp_types::definitions::MINOR,
            revision: fe2o3_amqp_types::definitions::REVISION,
        }
    }
}

impl ProtocolHeader {
    /// Creates a new protocol header
    pub fn new(id: ProtocolId, major: u8, minor: u8, revision: u8) -> Self {
        Self {
            id,
            major,
            minor,
            revision,
        }
    }

    /// Creates an AMQP protocol header
    pub fn amqp() -> Self {
        Self {
            id: ProtocolId::Amqp,
            ..Default::default()
        }
    }

    /// Returns whether the protocol id is AMQP
    pub fn is_amqp(&self) -> bool {
        match self.id {
            ProtocolId::Amqp => true,
            ProtocolId::Tls => false,
            ProtocolId::Sasl => false,
        }
    }

    /// Creates a TLS protocol header
    pub fn tls() -> Self {
        Self {
            id: ProtocolId::Tls,
            ..Default::default()
        }
    }

    /// Returns whether the protocol id is TLS
    pub fn is_tls(&self) -> bool {
        match self.id {
            ProtocolId::Amqp => false,
            ProtocolId::Tls => true,
            ProtocolId::Sasl => false,
        }
    }

    /// Creates a SASL protocol header
    pub fn sasl() -> Self {
        Self {
            id: ProtocolId::Sasl,
            ..Default::default()
        }
    }

    /// Returns whether the protocol id is SASL
    pub fn is_sasl(&self) -> bool {
        match self.id {
            ProtocolId::Amqp => false,
            ProtocolId::Tls => false,
            ProtocolId::Sasl => true,
        }
    }
}

impl From<ProtocolHeader> for [u8; 8] {
    fn from(value: ProtocolHeader) -> Self {
        [
            PROTOCOL_HEADER_PREFIX[0], // b'A'
            PROTOCOL_HEADER_PREFIX[1], // b'M'
            PROTOCOL_HEADER_PREFIX[2], // b'Q'
            PROTOCOL_HEADER_PREFIX[3], // b'P'
            value.id as u8,
            value.major,
            value.minor,
            value.revision,
        ]
    }
}

impl From<ProtocolHeader> for Bytes {
    fn from(header: ProtocolHeader) -> Self {
        let bytes: [u8; 8] = header.into();
        Bytes::copy_from_slice(&bytes[..])
    }
}

impl TryFrom<[u8; 8]> for ProtocolHeader {
    type Error = [u8; 8];

    fn try_from(v: [u8; 8]) -> Result<Self, Self::Error> {
        if &v[..4] != b"AMQP" {
            return Err(v);
        }
        let id = match v[4].try_into() {
            Ok(_id) => _id,
            Err(_) => return Err(v),
        };

        Ok(Self::new(id, v[5], v[6], v[7]))
    }
}

impl<'a> TryFrom<&'a [u8]> for ProtocolHeader {
    type Error = &'a [u8];

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if value.len() != 8 {
            return Err(value)
        }

        if value[..4] != b"AMQP"[..] {
            return Err(value)
        }

        let id = value[4].try_into()
            .map_err(|_| value)?;
        
        Ok(Self::new(id, value[5], value[6], value[7]))
    }
}

impl TryFrom<Bytes> for ProtocolHeader {
    type Error = Bytes;

    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        if value.len() != 8 {
            return Err(value)
        }

        if value[..4] != b"AMQP"[..] {
            return Err(value)
        }

        let id = match value[4].try_into() {
            Ok(id) => id,
            Err(_) => return Err(value),
        };
        
        Ok(Self::new(id, value[5], value[6], value[7]))
    }
}

/// Protocol ID
#[derive(Debug, Clone, PartialEq)]
pub enum ProtocolId {
    /// AMQP
    Amqp = 0x0,

    /// TLS
    Tls = 0x2,

    /// SASL
    Sasl = 0x3,
}

impl TryFrom<u8> for ProtocolId {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let val = match value {
            0x0 => Self::Amqp,
            0x2 => Self::Tls,
            0x3 => Self::Sasl,
            _ => return Err(value),
        };
        Ok(val)
    }
}

/// Encoder and Decoder for protocol headers
#[derive(Debug, Clone)]
pub struct ProtocolHeaderCodec {}

impl ProtocolHeaderCodec {
    /// Creates a new protocol header codec
    pub fn new() -> Self {
        Self {}
    }
}

impl Encoder<ProtocolHeader> for ProtocolHeaderCodec {
    type Error = io::Error;

    fn encode(&mut self, item: ProtocolHeader, dst: &mut bytes::BytesMut) -> Result<(), Self::Error> {
        let buf: [u8; 8] = item.into();
        dst.put(&buf[..]);
        Ok(())
    }
}

impl Decoder for ProtocolHeaderCodec {
    type Item = ProtocolHeader;
    type Error = NegotiationError;

    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // If the bytes look valid, but a frame isn’t fully available yet, then Ok(None) is
        // returned. This indicates to the Framed instance that it needs to read some more bytes
        // before calling this method again.
        if src.remaining() < 8 {
            return Ok(None)
        }

        let bytes = src.split_to(8).freeze();
        ProtocolHeader::try_from(bytes)
            .map(Some)
            .map_err(|b| NegotiationError::ProtocolHeaderMismatch(b))
    }
}