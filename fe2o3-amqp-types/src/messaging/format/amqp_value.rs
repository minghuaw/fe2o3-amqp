use std::fmt::Display;

use serde::{de, ser, Serialize};
use serde_amqp::{DeserializeComposite, SerializeComposite};

use crate::messaging::{
    AsBodyRef, DeserializableBody, FromBody, FromEmptyBody, IntoBody, SerializableBody,
    TransposeOption, __private::BodySection,
};

/// 3.2.8 AMQP Value
/// <type name="amqp-value" class="restricted" source="*" provides="section">
///     <descriptor name="amqp:amqp-value:*" code="0x00000000:0x00000077"/>
/// </type>
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, SerializeComposite, DeserializeComposite, Hash,
)]
#[amqp_contract(
    name = "amqp:amqp-value:*",
    code = "0x0000_0000:0x0000_0077",
    encoding = "basic"
)]
pub struct AmqpValue<T>(pub T);

impl<T> AmqpValue<T> {}

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
            Some(AmqpValue(Some(body))) => Some(T::from_body(AmqpValue(body))),
            Some(_) | None => None,
        }
    }
}

impl<'a, T: 'a> AsBodyRef<'a, T> for AmqpValue<T>
where
    T: IntoBody<Body = Self> + Serialize,
{
    type BodyRef = AmqpValue<&'a T>;

    fn as_body_ref(src: &'a T) -> Self::BodyRef {
        AmqpValue(src)
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_amqp::{from_slice, lazy::LazyValue, to_vec, Value};

    use crate::messaging::{
        message::__private::{Deserializable, Serializable},
        AmqpValue, Body, Message,
    };

    use super::{FromBody, FromEmptyBody, IntoBody};

    const TEST_STR: &str = "test_str";

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
    struct TestExample {
        a: i32,
    }

    impl IntoBody for TestExample {
        type Body = AmqpValue<Self>;

        fn into_body(self) -> Self::Body {
            AmqpValue(self)
        }
    }

    impl FromEmptyBody for TestExample {}

    impl<'de> FromBody<'de> for TestExample {
        type Body = AmqpValue<Self>;

        fn from_body(deserializable: Self::Body) -> Self {
            deserializable.0
        }
    }

    #[test]
    fn test_serde_custom_type() {
        let example = AmqpValue(TestExample { a: 9 });
        let buf = to_vec(&example).unwrap();
        let expected = [0x0, 0x53, 0x77, 0xc0, 0x3, 0x1, 0x54, 0x9];
        assert_eq!(buf, expected);
        let decoded: AmqpValue<TestExample> = from_slice(&buf).unwrap();
        assert_eq!(decoded.0.a, example.0.a);
    }

    #[test]
    fn test_encoding_option_str() {
        // let should_not_work = Message::builder()
        //     .body(Some(TEST_STR)) // This should NOT work
        //     .build();
        // let buf = to_vec(&Serializable(should_not_work)).unwrap();

        let expected_msg = Message::builder().value(TEST_STR).build();
        let expected = to_vec(&Serializable(expected_msg)).unwrap();

        let msg = Message::builder().value(Some(TEST_STR)).build();
        let buf = to_vec(&Serializable(msg)).unwrap();

        assert_eq!(buf, expected);
    }

    #[test]
    fn test_decoding_some_str() {
        let src = Message::builder().value(TEST_STR).build();
        let buf = to_vec(&Serializable(src)).unwrap();
        let msg: Deserializable<Message<Option<String>>> = from_slice(&buf).unwrap();

        assert!(msg.0.body.is_some());
        assert_eq!(msg.0.body.unwrap(), TEST_STR);
    }

    #[test]
    fn test_decoding_none_str_with_body_section_value_null() {
        let src = Message::builder()
            // This actually serializes to `AmpqValue(Value::Null)`
            .body(Body::<Value>::Empty)
            .build();
        let buf = to_vec(&Serializable(src)).unwrap();
        // This will give an error if the empty body section is not encoded as a
        // `AmqpValue(Value::Null)`
        let msg: Deserializable<Message<Option<String>>> = from_slice(&buf).unwrap();

        assert!(msg.0.body.is_none());
    }

    #[test]
    fn test_decoding_none_str_with_no_body_section() {
        let buf: [u8; 8] = [0x0, 0x53, 0x70, 0x45, 0x0, 0x53, 0x73, 0x45];
        let msg: Deserializable<Message<Option<String>>> = from_slice(&buf).unwrap();
        assert!(msg.0.header.is_some());
        assert!(msg.0.body.is_none());
    }

    #[test]
    fn test_encoding_decoding_custom_type() {
        let expected = TestExample { a: 9 };
        let msg = Message::from(expected.clone());
        let buf = to_vec(&Serializable(msg)).unwrap();
        let decoded: Deserializable<Message<TestExample>> = from_slice(&buf).unwrap();
        assert_eq!(decoded.0.body, expected)
    }

    #[test]
    fn test_encoding_reference() {
        let expected = TestExample { a: 9 };
        let msg = Message::from(&expected);

        let buf = to_vec(&Serializable(msg)).unwrap();
        let decoded: Deserializable<Message<TestExample>> = from_slice(&buf).unwrap();
        assert_eq!(decoded.0.body, expected)
    }

    #[test]
    fn test_decoding_some_str_as_lazy_value() {
        let src = Message::builder().value(TEST_STR).build();
        let buf = to_vec(&Serializable(src)).unwrap();
        let msg: Deserializable<Message<AmqpValue<LazyValue>>> = from_slice(&buf).unwrap();

        let expected = to_vec(&TEST_STR).unwrap();
        assert_eq!(msg.0.body.0.as_slice(), expected);
    }
}
