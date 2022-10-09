use std::fmt::Display;

use serde::{de, Serialize};
use serde_amqp::{primitives::Binary, Value, SerializeComposite, DeserializeComposite};

use crate::messaging::{message::__private::{Deserializable, Serializable}, SerializableBody, sealed::Sealed, Batch, DeserializableBody};

/// 3.2.6 Data
/// <type name="data" class="restricted" source="binary" provides="section">
///     <descriptor name="amqp:data:binary" code="0x00000000:0x00000075"/>
/// </type>
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name="amqp:data:binary",
    code=0x0000_0000_0000_0075,
    encoding = "basic",
)]
pub struct Data(pub Binary);

impl TryFrom<Value> for Data {
    type Error = Value;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Binary(buf) = value {
            Ok(Data(buf))
        } else {
            Err(value)
        }
    }
}

impl Display for Data {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Data of length: {}", self.0.len())
    }
}

// impl Serialize for Serializable<Data> {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//     {
//         use serde_amqp::serde::ser::SerializeTupleStruct;
//         let mut state = serializer
//             .serialize_tuple_struct(serde_amqp::__constants::DESCRIBED_BASIC, 1usize + 1)?;
//         state.serialize_field(&serde_amqp::descriptor::Descriptor::Code(
//             0x0000_0000_0000_0075_u64,
//         ))?;
//         state.serialize_field(&self.0 .0)?;
//         state.end()
//     }
// }

// impl Serialize for Serializable<&Data> {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//     {
//         use serde_amqp::serde::ser::SerializeTupleStruct;
//         let mut state = serializer
//             .serialize_tuple_struct(serde_amqp::__constants::DESCRIBED_BASIC, 1usize + 1)?;
//         state.serialize_field(&serde_amqp::descriptor::Descriptor::Code(
//             0x0000_0000_0000_0075_u64,
//         ))?;
//         state.serialize_field(&self.0 .0)?;
//         state.end()
//     }
// }

// impl<'de> de::Deserialize<'de> for Deserializable<Data> {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: serde::Deserializer<'de>,
//     {
//         struct Visitor {}

//         impl Visitor {
//             fn new() -> Self {
//                 Self {}
//             }
//         }

//         impl<'de> serde_amqp::serde::de::Visitor<'de> for Visitor {
//             type Value = Deserializable<Data>;

//             fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
//                 formatter.write_str("amqp:data:binary")
//             }

//             fn visit_seq<A>(self, mut __seq: A) -> Result<Self::Value, A::Error>
//             where
//                 A: serde_amqp::serde::de::SeqAccess<'de>,
//             {
//                 let __descriptor: serde_amqp::descriptor::Descriptor = match __seq.next_element()? {
//                     Some(val) => val,
//                     None => {
//                         return Err(serde_amqp::serde::de::Error::custom("Expecting descriptor"))
//                     }
//                 };
//                 match __descriptor {
//                     serde_amqp::descriptor::Descriptor::Name(__symbol) => {
//                         if __symbol.into_inner() != "amqp:data:binary" {
//                             return Err(serde_amqp::serde::de::Error::custom(
//                                 "Descriptor mismatch",
//                             ));
//                         }
//                     }
//                     serde_amqp::descriptor::Descriptor::Code(__c) => {
//                         if __c != 0x0000_0000_0000_0075_u64 {
//                             return Err(serde_amqp::serde::de::Error::custom(
//                                 "Descriptor mismatch",
//                             ));
//                         }
//                     }
//                 }
//                 let field0: Binary = match __seq.next_element()? {
//                     Some(val) => val,
//                     None => {
//                         return Err(serde_amqp::serde::de::Error::custom(
//                             "Insufficient number of items",
//                         ))
//                     }
//                 };
//                 Ok(Deserializable(Data(field0)))
//             }
//         }

//         deserializer.deserialize_tuple_struct(
//             serde_amqp::__constants::DESCRIBED_BASIC,
//             1usize + 1,
//             Visitor::new(),
//         )
//     }
// }

impl Sealed for Data {}

impl<'se> Sealed for &'se Data {}

impl Sealed for Batch<Data> { }

impl<'se> Sealed for Batch<&'se Data> { }