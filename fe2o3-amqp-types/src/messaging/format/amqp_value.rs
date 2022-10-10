use std::fmt::Display;

use serde::{de, ser, Serialize};
use serde_amqp::{DeserializeComposite, SerializeComposite};

use crate::messaging::{
    DeserializableBody, FromBody, FromEmptyBody, IntoBody, SerializableBody, TransposeOption,
    __private::BodySection,
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

impl<T> IntoBody for AmqpValue<T>
where
    T: ser::Serialize,
{
    type Body = Self;

    fn into_body(self) -> Self::Body {
        self
    }
}

impl<'de, T> FromBody<'de> for AmqpValue<T>
where
    T: de::Deserialize<'de> + FromEmptyBody,
{
    type Body = Self;

    fn from_body(deserializable: Self::Body) -> Self {
        deserializable
    }
}

impl<T> FromEmptyBody for AmqpValue<T>
where
    T: FromEmptyBody,
{
    fn from_empty_body() -> Result<Self, serde_amqp::Error> {
        T::from_empty_body().map(AmqpValue)
    }
}

impl<'de, T, U> TransposeOption<'de, T> for AmqpValue<U>
where
    T: FromBody<'de, Body = AmqpValue<U>>,
    U: de::Deserialize<'de>,
{
    type From = Option<AmqpValue<Option<U>>>;

    fn transpose(src: Self::From) -> Option<T> {
        match src {
            Some(AmqpValue(body)) => match body {
                Some(body) => Some(T::from_body(AmqpValue(body))),
                None => None,
            },
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_amqp::{from_slice, to_vec};

    use crate::messaging::{
        message::__private::{Deserializable, Serializable},
        AmqpValue,
    };

    #[derive(Debug, Serialize, Deserialize)]
    struct TestExample {
        a: i32,
    }

    #[test]
    fn test_serde_custom_type() {
        let example = Serializable(AmqpValue(TestExample { a: 9 }));
        let buf = to_vec(&example).unwrap();
        let expected = [0x0, 0x53, 0x77, 0xc0, 0x3, 0x1, 0x54, 0x9];
        assert_eq!(buf, expected);
        let decoded: Deserializable<AmqpValue<TestExample>> = from_slice(&buf).unwrap();
        assert_eq!(decoded.0 .0.a, example.0 .0.a);
    }
}
