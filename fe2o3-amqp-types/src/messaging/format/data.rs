use std::{borrow::Cow, fmt::Display};

use serde_amqp::{primitives::Binary, DeserializeComposite, SerializeComposite, Value};

use crate::messaging::{
    Batch, DeserializableBody, FromBody, FromEmptyBody, IntoBody, SerializableBody,
    TransposeOption, __private::BodySection,
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

impl<'de, T> TransposeOption<'de, T> for Data
where
    T: FromBody<'de, Body = Data>,
{
    type From = Option<Data>;

    fn transpose(src: Self::From) -> Option<T> {
        match src {
            Some(data) => {
                if data.0.is_empty() {
                    // Note that a null value and a zero-length array (with a correct type for its
                    // elements) both describe an absence of a value and MUST be treated as
                    // semantically identical.
                    None
                } else {
                    Some(T::from_body(data))
                }
            }
            None => None,
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                                    Data                                    */
/* -------------------------------------------------------------------------- */

impl BodySection for Data {}

impl SerializableBody for Data {}

impl<'de> DeserializableBody<'de> for Data {}

impl IntoBody for Data {
    type Body = Self;

    fn into_body(self) -> Self::Body {
        self
    }
}

impl<'de> FromBody<'de> for Data {
    type Body = Data;

    fn from_body(deserializable: Self::Body) -> Self {
        deserializable
    }
}

impl FromEmptyBody for Data {}

/* -------------------------------------------------------------------------- */
/*                                 Batch<Data>                                */
/* -------------------------------------------------------------------------- */

impl BodySection for Batch<Data> {}

impl SerializableBody for Batch<Data> {}

impl<'de> DeserializableBody<'de> for Batch<Data> {}

impl IntoBody for Batch<Data> {
    type Body = Self;

    fn into_body(self) -> Self::Body {
        self
    }
}

impl<'de> FromBody<'de> for Batch<Data> {
    type Body = Batch<Data>;

    fn from_body(deserializable: Self::Body) -> Self {
        deserializable
    }
}

impl FromEmptyBody for Batch<Data> {}

impl<'de, T> TransposeOption<'de, T> for Batch<Data>
where
    T: FromBody<'de, Body = Batch<Data>>,
{
    type From = Option<Batch<Data>>;

    fn transpose(src: Self::From) -> Option<T> {
        match src {
            Some(batch) => {
                if batch.is_empty() {
                    None
                } else {
                    Some(T::from_body(batch))
                }
            }
            None => None,
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                                    Test                                    */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use serde_amqp::{from_slice, to_vec};

    use crate::messaging::{
        message::__private::{Deserializable, Serializable},
        Message,
    };

    use super::Data;

    const TEST_STR: &str = "this is some random string that acts as a test str";

    #[test]
    fn test_serde_data() {
        let msg = Message::builder().data(TEST_STR.as_bytes()).build();
        let buf = to_vec(&Serializable(msg)).unwrap();
        let decoded: Deserializable<Message<Data>> = from_slice(&buf).unwrap();
        assert_eq!(decoded.0.body.0, TEST_STR.as_bytes());
    }
}
