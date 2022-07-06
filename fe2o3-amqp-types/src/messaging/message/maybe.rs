//! Implements a workaround when no body section is found in message

use std::{marker::PhantomData, io};

use serde::{Serialize, de};
use serde_amqp::__constants::{DESCRIBED_BASIC, DESCRIPTOR};

use crate::messaging::{Data, AmqpSequence, AmqpValue};

use super::{__private::{Deserializable}, Message, Visitor, Body, DecodeIntoMessage};

/// A wrapper type that allows deserializing a message that doesn't have any body section.
/// 
/// The core specification states that **at least one** body section should be present in
/// the message. However, this is not the way `proton` is implemented, and according to 
/// [PROTON-2574](https://issues.apache.org/jira/browse/PROTON-2574), the wording in the 
/// core specification was an unintended.
/// 
/// Instead of simply setting the `body` field to an `Option<Body<T>>`, this approach is
/// implemented chosen to allow users to decide what to do when there is no body section.
/// 
/// # Example
/// 
/// ```rust, ignore
/// 
/// let buf: [u8; 8] = [ 0x0, 0x53, 0x70, 0x45, 0x0, 0x53, 0x73, 0x45 ];
/// let result: Result<Deserializable<Message<Value>>, _> = from_slice(&buf);
/// assert!(result.is_err());
/// 
/// let result: Result<Deserializable<Message<Maybe<Value>>>, _> = from_slice(&buf);
/// assert!(result.is_ok());
/// let message = result.unwrap().0;
/// println!("{:?}", message);
/// ```
#[derive(Debug)]
pub enum Maybe<T> {
    /// The body section is present
    Just(T),

    /// The body section is not present
    Nothing
}

impl<T> From<Maybe<T>> for Option<T> {
    fn from(value: Maybe<T>) -> Self {
        match value {
            Maybe::Just(v) => Some(v),
            Maybe::Nothing => None,
        }
    }
}

impl<T> From<Option<T>> for Maybe<T> {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(v) => Self::Just(v),
            None => Self::Nothing,
        }
    }
}

impl<T> Maybe<T> {
    /// Converts into an `Option<T>`
    pub fn into_option(self) -> Option<T> {
        self.into()
    }

    /// Transforms the Maybe<T> into a Result<T, E>, mapping Just(v) to Ok(v) and Nothing to Err(err).
    pub fn ok_or<E>(self, err: E) -> Result<T, E> {
        match self {
            Maybe::Just(v) => Ok(v),
            Maybe::Nothing => Err(err),
        }
    }

    /// Transforms the `Maybe<T>` into a `Result<T, E>`, mapping Just(v) to Ok(v) and `Nothing` to `Err(err())`.
    pub fn ok_or_else<E, F>(self, err: F) -> Result<T, E> 
    where
        F: FnOnce() -> E
    {
        match self {
            Maybe::Just(v) => Ok(v),
            Maybe::Nothing => Err(err()),
        }
    }
}

impl<T: Serialize> Serialize for Maybe<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        match self {
            Maybe::Just(value) => value.serialize(serializer),
            Maybe::Nothing => ().serialize(serializer),
        }
    }
}

impl<'de, T> de::Visitor<'de> for Visitor<Maybe<T>> 
where
    T: de::Deserialize<'de>,
{
    type Value = Message<Maybe<T>>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("struct Message")
    }

    
    fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let visitor = Visitor::<T> { marker: PhantomData };
        let (header,
            delivery_annotations,
            message_annotations,
            properties,
            application_properties,
            body,
            footer) = visitor.visit_fields(seq)?;

        let body = match body {
            Some(Deserializable(body)) => match body {
                Body::Data(Data(data)) => Body::<Maybe<T>>::Data(Data(data)),
                Body::Sequence(AmqpSequence(seq)) => Body::<Maybe<T>>::Sequence(AmqpSequence(seq.into_iter().map(Maybe::Just).collect())),
                Body::Value(AmqpValue(value)) => Body::<Maybe<T>>::Value(AmqpValue(Maybe::Just(value))),
            },
            None => Body::<Maybe<T>>::Value(AmqpValue(Maybe::Nothing)),
        };

        Ok(Message {
            header,
            delivery_annotations,
            message_annotations,
            properties,
            application_properties,
            body,
            footer,
        })
    }
}

impl<'de, T> Message<Maybe<T>>
where
    T: de::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as serde::Deserializer<'de>>::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_struct(
            DESCRIBED_BASIC,
            &[
                DESCRIPTOR,
                "header",
                DESCRIPTOR,
                "delivery_annotations",
                DESCRIPTOR,
                "message_annotations",
                DESCRIPTOR,
                "properties",
                DESCRIPTOR,
                "application_properties",
                DESCRIPTOR,
                "body",
                DESCRIPTOR,
                "footer",
            ],
            Visitor::<Maybe<T>> {
                marker: PhantomData,
            },
        )
    }
}

impl<'de, T> de::Deserialize<'de> for Deserializable<Message<Maybe<T>>>
where
    T: de::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Message::<Maybe<T>>::deserialize(deserializer)?;
        Ok(Deserializable(value))
    }
}

impl<T> DecodeIntoMessage for Maybe<T> where for<'de> T: de::Deserialize<'de> {
    type DecodeError = serde_amqp::Error;

    fn decode_into_message(reader: impl io::Read) -> Result<Message<Self>, Self::DecodeError> {
        let message: Deserializable<Message<Maybe<T>>> = serde_amqp::from_reader(reader)?;
        Ok(message.0)
    }
}

#[cfg(test)]
mod tests {
    use serde_amqp::{from_slice, Value};

    use crate::messaging::{Message, message::{__private::Deserializable, maybe::Maybe}};

    #[test]
    fn test_decoding_message_with_no_body_section() {
        let buf: [u8; 8] = [ 0x0, 0x53, 0x70, 0x45, 0x0, 0x53, 0x73, 0x45 ];
        let result: Result<Deserializable<Message<Value>>, _> = from_slice(&buf);
        assert!(result.is_err());

        let result: Result<Deserializable<Message<Maybe<Value>>>, _> = from_slice(&buf);
        assert!(result.is_ok());
        let message = result.unwrap().0;
        println!("{:?}", message);
    }
}