use std::fmt::Display;

use serde::{de, ser, Serialize};
use serde_amqp::{DeserializeComposite, SerializeComposite};

use crate::messaging::{
    DeserializableBody, FromDeserializableBody, FromEmptyBody, IntoSerializableBody,
    SerializableBody, __private::BodySection,
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

impl<T> BodySection for AmqpValue<T> {}

impl<T> SerializableBody for AmqpValue<T> where T: Serialize {}

impl<'de, T> DeserializableBody<'de> for AmqpValue<T> where T: de::Deserialize<'de> {}

impl<T> IntoSerializableBody for AmqpValue<T>
where
    T: ser::Serialize,
{
    type SerializableBody = Self;

    fn into_serializable_body(self) -> Self::SerializableBody {
        self
    }
}

impl<'de, T> FromDeserializableBody<'de> for AmqpValue<T>
where
    T: de::Deserialize<'de> + FromEmptyBody,
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
