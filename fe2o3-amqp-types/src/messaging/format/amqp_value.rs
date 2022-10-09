use std::fmt::Display;

use serde::{de, ser};
use serde_amqp::{DeserializeComposite, SerializeComposite};

use crate::messaging::{
    sealed::Sealed, DeserializableBody, FromDeserializableBody, FromEmptyBody,
    IntoSerializableBody, SerializableBody,
};

/// 3.2.8 AMQP Value
/// <type name="amqp-value" class="restricted" source="*" provides="section">
///     <descriptor name="amqp:amqp-value:*" code="0x00000000:0x00000077"/>
/// </type>
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, SerializeComposite, DeserializeComposite,
)]
#[amqp_contract(
    name = "amqp:amqp-value:*",
    code = 0x0000_0000_0000_0077,
    encoding = "basic"
)]
pub struct AmqpValue<T>(pub T);

impl<T> Display for AmqpValue<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AmqpValue({})", self.0)
    }
}

impl<T> Sealed for AmqpValue<T> {}

impl<'se, T> Sealed for &'se AmqpValue<T> {}

impl<T> SerializableBody for AmqpValue<T>
where
    T: ser::Serialize,
{
    type Serializable = Self;

    fn serializable(&self) -> &Self::Serializable {
        self
    }
}

impl<T> DeserializableBody for AmqpValue<T>
where
    for<'de> T: de::Deserialize<'de>,
{
    type Deserializable = Self;

    fn from_deserializable(deserializable: Self::Deserializable) -> Self {
        deserializable
    }
}

impl<T> IntoSerializableBody for AmqpValue<T>
where
    T: ser::Serialize,
{
    type SerializableBody = Self;

    fn into_serializable_body(self) -> Self::SerializableBody {
        self
    }
}

impl<T> FromDeserializableBody for AmqpValue<T>
where
    for<'de> T: de::Deserialize<'de> + FromEmptyBody,
{
    type DeserializableBody = Self;

    fn from_deserializable_body(deserializable: Self::DeserializableBody) -> Self {
        deserializable
    }
}

impl<T> FromEmptyBody for AmqpValue<T>
where
    T: FromEmptyBody,
{
    type Error = T::Error;

    fn from_empty_body() -> Result<Self, Self::Error> {
        T::from_empty_body().map(AmqpValue)
    }
}
