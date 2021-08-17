use serde::{Serializer, de};
use serde::ser::{Serialize, SerializeStruct};

use crate::types::Symbol;

pub const DESCRIPTOR: &str = "DESCRIPTOR";

/// Descriptor of a described type
///
/// How are descriptor name serilaized in other implementations?
/// 1. amqpnetlite: Symbol
/// 2. go-amqp: Symbol?
/// 3. qpid-proton-j2: Symbol
#[derive(Debug)]
pub enum Descriptor {
    Name(Symbol),
    Code(u64)
}

impl Descriptor {
    pub fn name(name: impl Into<Symbol>) -> Self {
        Self::Name(name.into())
    }

    pub fn code(code: u64) -> Self {
        Self::Code(code.into())
    }
}

impl Serialize for Descriptor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Descriptor::Name(value) => {
                serializer.serialize_newtype_variant(DESCRIPTOR, 0, "Name", value)
            },
            Descriptor::Code(value) => {
                serializer.serialize_newtype_variant(DESCRIPTOR, 1, "Code", value)
            }
        }
    }
}

// impl<'de> de::Deserialize<'de> for Descriptor {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: serde::Deserializer<'de>
//     {
//         enum Field {
//             name:
//         }
//     }
// }
