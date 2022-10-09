use std::{fmt::Display, borrow::Cow};

use serde_amqp::{primitives::Binary, DeserializeComposite, SerializeComposite, Value};

use crate::messaging::{
    sealed::Sealed, Batch, DeserializableBody, FromDeserializableBody, FromEmptyBody,
    IntoSerializableBody, SerializableBody,
};

/// 3.2.6 Data
/// <type name="data" class="restricted" source="binary" provides="section">
///     <descriptor name="amqp:data:binary" code="0x00000000:0x00000075"/>
/// </type>
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, SerializeComposite, DeserializeComposite,
)]
#[amqp_contract(
    name = "amqp:data:binary",
    code = 0x0000_0000_0000_0075,
    encoding = "basic"
)]
pub struct Data(pub Binary);

impl From<Binary> for Data {
    fn from(value: Binary) -> Self {
        Self(value)
    }
}

impl From<Vec<u8>> for Data {
    fn from(value: Vec<u8>) -> Self {
        Self(Binary::from(value))
    }
}

impl<const N: usize> From<[u8; N]> for Data {
    fn from(value: [u8; N]) -> Self {
        Self(Binary::from(value))
    }
}

impl From<&[u8]> for Data {
    fn from(value: &[u8]) -> Self {
        Self(Binary::from(value))
    }
}

impl<'a> From<Cow<'a, [u8]>> for Data {
    fn from(value: Cow<'a, [u8]>) -> Self {
        Self(Binary::from(value.to_vec()))
    }
}

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

/* -------------------------------------------------------------------------- */
/*                                    Data                                    */
/* -------------------------------------------------------------------------- */

impl Sealed for Data {}

impl<'se> Sealed for &'se Data {}

impl SerializableBody for Data {
    type Serializable = Self;

    fn serializable(&self) -> &Self::Serializable {
        self
    }
}

impl DeserializableBody for Data {
    type Deserializable = Self;

    fn from_deserializable(deserializable: Self::Deserializable) -> Self {
        deserializable
    }
}

impl IntoSerializableBody for Data {
    type SerializableBody = Self;

    fn into_serializable_body(self) -> Self::SerializableBody {
        self
    }
}

impl FromDeserializableBody for Data {
    type DeserializableBody = Data;

    fn from_deserializable_body(deserializable: Self::DeserializableBody) -> Self {
        deserializable
    }
}

impl FromEmptyBody for Data {
    type Error = serde_amqp::Error;
}

/* -------------------------------------------------------------------------- */
/*                                 Batch<Data>                                */
/* -------------------------------------------------------------------------- */

impl Sealed for Batch<Data> {}

impl<'se> Sealed for Batch<&'se Data> {}

impl SerializableBody for Batch<Data> {
    type Serializable = Self;

    fn serializable(&self) -> &Self::Serializable {
        self
    }
}

impl DeserializableBody for Batch<Data> {
    type Deserializable = Self;

    fn from_deserializable(deserializable: Self::Deserializable) -> Self {
        deserializable
    }
}

impl IntoSerializableBody for Batch<Data> {
    type SerializableBody = Self;

    fn into_serializable_body(self) -> Self::SerializableBody {
        self
    }
}

impl FromDeserializableBody for Batch<Data> {
    type DeserializableBody = Batch<Data>;

    fn from_deserializable_body(deserializable: Self::DeserializableBody) -> Self {
        deserializable
    }
}

impl FromEmptyBody for Batch<Data> {
    type Error = serde_amqp::Error;
}
