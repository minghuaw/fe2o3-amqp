//! Implements the protocol headers

use std::convert::{TryFrom, TryInto};

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

    /// Creates a TLS protocol header
    pub fn tls() -> Self {
        Self {
            id: ProtocolId::Tls,
            ..Default::default()
        }
    }

    /// Creates a SASL protocol header
    pub fn sasl() -> Self {
        Self {
            id: ProtocolId::Sasl,
            ..Default::default()
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
