use std::fmt::Display;

use serde::{de, ser, Serialize};
use serde_amqp::{DeserializeComposite, SerializeComposite};

use crate::messaging::{
    Batch, DeserializableBody, FromBody, FromEmptyBody, IntoBody, SerializableBody,
    TransposeOption, __private::BodySection,
};

/// 3.2.7 AMQP Sequence
/// <type name="amqp-sequence" class="restricted" source="list" provides="section">
///     <descriptor name="amqp:amqp-sequence:list" code="0x00000000:0x00000076"/>
/// </type>
#[derive(
    Debug,
    Clone,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    SerializeComposite,
    DeserializeComposite,
    Hash,
)]
#[amqp_contract(
    name = "amqp:amqp-sequence:list",
    code = "0x0000_0000:0x0000_0076",
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

impl<T> From<Vec<T>> for AmqpSequence<T> {
    fn from(value: Vec<T>) -> Self {
        Self(value)
    }
}

/* -------------------------------------------------------------------------- */
/*                                AmqpSequence                                */
/* -------------------------------------------------------------------------- */

impl<T> BodySection for AmqpSequence<T> {}

impl<T> SerializableBody for AmqpSequence<T> where T: Serialize {}

impl<'de, T> DeserializableBody<'de> for AmqpSequence<T> where T: de::Deserialize<'de> {}

impl<T> IntoBody for AmqpSequence<T>
where
    T: ser::Serialize,
{
    type Body = Self;

    fn into_body(self) -> Self::Body {
        self
    }
}

impl<'de, T> FromBody<'de> for AmqpSequence<T>
where
    T: de::Deserialize<'de>,
{
    type Body = Self;

    fn from_body(deserializable: Self::Body) -> Self {
        deserializable
    }
}

impl<T> FromEmptyBody for AmqpSequence<T> {}

impl<'de, T, U> TransposeOption<'de, T> for AmqpSequence<U>
where
    T: FromBody<'de, Body = AmqpSequence<U>>,
    U: de::Deserialize<'de>,
{
    type From = Option<AmqpSequence<Option<U>>>;

    fn transpose(src: Self::From) -> Option<T> {
        match src {
            Some(AmqpSequence(vec)) => {
                let vec: Option<Vec<U>> = vec.into_iter().collect();
                match vec {
                    Some(vec) => {
                        if vec.is_empty() {
                            // Note that a null value and a zero-length array (with a correct type
                            // for its elements) both describe an absence of a value and MUST be
                            // treated as semantically identical.
                            None
                        } else {
                            Some(T::from_body(AmqpSequence(vec)))
                        }
                    }
                    None => None,
                }
            }
            None => None,
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                             Batch<AmqpSequece>                             */
/* -------------------------------------------------------------------------- */

impl<T> BodySection for Batch<AmqpSequence<T>> {}

impl<T> SerializableBody for Batch<AmqpSequence<T>> where T: Serialize {}

impl<'de, T> DeserializableBody<'de> for Batch<AmqpSequence<T>> where T: de::Deserialize<'de> {}

impl<T> IntoBody for Batch<AmqpSequence<T>>
where
    T: ser::Serialize,
{
    type Body = Self;

    fn into_body(self) -> Self::Body {
        self
    }
}

impl<'de, T> FromBody<'de> for Batch<AmqpSequence<T>>
where
    T: de::Deserialize<'de>,
{
    type Body = Self;

    fn from_body(deserializable: Self::Body) -> Self {
        deserializable
    }
}

impl<T> FromEmptyBody for Batch<AmqpSequence<T>> {}

impl<'de, T, U> TransposeOption<'de, T> for Batch<AmqpSequence<U>>
where
    T: FromBody<'de, Body = Batch<AmqpSequence<U>>>,
    U: de::Deserialize<'de>,
{
    type From = Option<Batch<AmqpSequence<Option<U>>>>;

    fn transpose(src: Self::From) -> Option<T> {
        match src {
            Some(batch) => {
                if batch.is_empty() {
                    return None;
                }

                let batch: Option<Batch<AmqpSequence<U>>> = batch
                    .into_iter()
                    .map(|AmqpSequence(vec)| {
                        let vec: Option<Vec<U>> = vec.into_iter().collect();
                        vec.map(AmqpSequence)
                    })
                    .collect();

                batch.map(T::from_body)
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
    use serde::{Deserialize, Serialize};
    use serde_amqp::{from_slice, to_vec};

    use crate::messaging::{
        message::__private::{Deserializable, Serializable},
        AmqpSequence, Batch, Message,
    };

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
    struct TestExample {
        a: i32,
    }

    #[test]
    fn test_serde_amqp_sequence() {
        let examples = vec![
            TestExample { a: 1 },
            TestExample { a: 2 },
            TestExample { a: 3 },
        ];
        let message = Message::builder()
            // Unless wrapped inside a `AmqpSequence`, a `Vec` will be serialized as `AmqpValue`
            .sequence(examples.clone())
            .build();
        let buf = to_vec(&Serializable(message)).unwrap();
        let decoded: Deserializable<Message<AmqpSequence<TestExample>>> = from_slice(&buf).unwrap();
        assert_eq!(decoded.0.body.0, examples);
    }

    #[test]
    fn test_serde_amqp_sequence_with_ref() {
        let example_1 = TestExample { a: 1 };
        let example_2 = TestExample { a: 2 };
        let example_3 = TestExample { a: 3 };
        let examples = vec![&example_1, &example_2, &example_3];
        let message = Message::builder()
            // Unless wrapped inside a `AmqpSequence`, a `Vec` will be serialized as `AmqpValue`
            .sequence(examples.clone())
            .build();
        let buf = to_vec(&Serializable(message)).unwrap();
        let decoded: Deserializable<Message<AmqpSequence<TestExample>>> = from_slice(&buf).unwrap();
        assert_eq!(decoded.0.body.0, vec![example_1, example_2, example_3]);
    }

    #[test]
    fn test_serde_amqp_sequence_batch() {
        let examples = vec![
            TestExample { a: 1 },
            TestExample { a: 2 },
            TestExample { a: 3 },
        ];
        let msg = Message::builder()
            .sequence_batch(vec![examples.clone(), examples.clone(), examples.clone()])
            .build();
        let buf = to_vec(&Serializable(msg)).unwrap();
        let decoded: Deserializable<Message<Batch<AmqpSequence<TestExample>>>> =
            from_slice(&buf).unwrap();

        let expected: Vec<AmqpSequence<TestExample>> =
            vec![examples.clone(), examples.clone(), examples.clone()]
                .into_iter()
                .map(Into::into)
                .collect();
        assert_eq!(decoded.0.body.into_inner(), expected);
    }
}
