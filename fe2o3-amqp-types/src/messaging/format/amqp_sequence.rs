use std::fmt::Display;

use serde::{de, ser, Serialize};
use serde_amqp::{DeserializeComposite, SerializeComposite};

use crate::{messaging::{
    Batch, DeserializableBody, FromDeserializableBody, FromEmptyBody,
    IntoSerializableBody, SerializableBody, BodySection,
}, __sealed::Sealed};

/// 3.2.7 AMQP Sequence
/// <type name="amqp-sequence" class="restricted" source="list" provides="section">
///     <descriptor name="amqp:amqp-sequence:list" code="0x00000000:0x00000076"/>
/// </type>
#[derive(
    Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, SerializeComposite, DeserializeComposite,
)]
#[amqp_contract(
    name = "amqp:amqp-sequence:list",
    code = 0x0000_0000_0000_0076,
    encoding = "basic"
)]
pub struct AmqpSequence<T>(pub Vec<T>); // Vec doesnt implement Display trait

impl<T> AmqpSequence<T> {
    /// Creates a new [`AmqpSequence`]
    pub fn new(vec: Vec<T>) -> Self {
        Self(vec)
    }
}

impl<T> Display for AmqpSequence<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AmqpSequence([")?;
        let len = self.0.len();
        for (i, val) in self.0.iter().enumerate() {
            write!(f, "{}", val)?;
            if i < len - 1 {
                f.write_str(", ")?;
            }
        }
        f.write_str("])")
    }
}

/* -------------------------------------------------------------------------- */
/*                                AmqpSequence                                */
/* -------------------------------------------------------------------------- */

impl<T> Sealed for AmqpSequence<T> {}

impl<T> BodySection for AmqpSequence<T> {}

impl<'se, T> Sealed for &'se AmqpSequence<T> {}

impl<'se, T> BodySection for &'se AmqpSequence<T> {}

impl<T> SerializableBody for AmqpSequence<T> where T: Serialize {}

impl<T> DeserializableBody for AmqpSequence<T>
where
    for<'de> T: de::Deserialize<'de>,
{
    type Deserializable = Self;

    fn from_deserializable(deserializable: Self::Deserializable) -> Self {
        deserializable
    }
}

impl<T> IntoSerializableBody for AmqpSequence<T>
where
    T: ser::Serialize,
{
    type SerializableBody = Self;

    fn into_serializable_body(self) -> Self::SerializableBody {
        self
    }
}

impl<T> FromDeserializableBody for AmqpSequence<T>
where
    for<'de> T: de::Deserialize<'de>,
{
    type DeserializableBody = Self;

    fn from_deserializable_body(deserializable: Self::DeserializableBody) -> Self {
        deserializable
    }
}

impl<T> FromEmptyBody for AmqpSequence<T> {
    type Error = serde_amqp::Error;
}

/* -------------------------------------------------------------------------- */
/*                             Batch<AmqpSequece>                             */
/* -------------------------------------------------------------------------- */

impl<T> Sealed for Batch<AmqpSequence<T>> {}

impl<T> BodySection for Batch<AmqpSequence<T>> {}

impl<T> SerializableBody for Batch<AmqpSequence<T>> where T: Serialize {}

impl<T> DeserializableBody for Batch<AmqpSequence<T>>
where
    for<'de> T: de::Deserialize<'de>,
{
    type Deserializable = Self;

    fn from_deserializable(deserializable: Self::Deserializable) -> Self {
        deserializable
    }
}

impl<T> IntoSerializableBody for Batch<AmqpSequence<T>>
where
    T: ser::Serialize,
{
    type SerializableBody = Self;

    fn into_serializable_body(self) -> Self::SerializableBody {
        self
    }
}

impl<T> FromDeserializableBody for Batch<AmqpSequence<T>>
where
    for<'de> T: de::Deserialize<'de>,
{
    type DeserializableBody = Self;

    fn from_deserializable_body(deserializable: Self::DeserializableBody) -> Self {
        deserializable
    }
}

impl<T> FromEmptyBody for Batch<AmqpSequence<T>> {
    type Error = serde_amqp::Error;
}

impl<'se, T> Sealed for Batch<&'se AmqpSequence<T>> {}

impl<'se, T> BodySection for Batch<&'se AmqpSequence<T>> {}