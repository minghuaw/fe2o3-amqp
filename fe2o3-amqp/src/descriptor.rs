use serde::de;
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
pub struct Descriptor {
    name: Option<Symbol>,
    code: Option<u64>,
}

impl Descriptor {
    pub fn new<T: Into<Symbol>>(name: Option<T>, code: Option<u64>) -> Self {
        Self {
            name: name.map(Into::into),
            code,
        }
    }
}

impl Serialize for Descriptor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct(DESCRIPTOR, 1)?;
        if let Some(code) = &self.code {
            state.serialize_field("code", code)?;
        } else if let Some(name) = &self.name {
            state.serialize_field("name", name)?;
        } else {
            state.serialize_field("name", &Symbol::new("".to_string()))?;
        }
        state.end()
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
