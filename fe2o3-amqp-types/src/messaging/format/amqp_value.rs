use std::fmt::Display;

use serde::{de, Deserialize, Serialize};

use crate::messaging::message::__private::{Deserializable, Serializable};

/// 3.2.8 AMQP Value
/// <type name="amqp-value" class="restricted" source="*" provides="section">
///     <descriptor name="amqp:amqp-value:*" code="0x00000000:0x00000077"/>
/// </type>
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AmqpValue<T>(pub T);

impl<T> Display for AmqpValue<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AmqpValue({})", self.0)
    }
}

impl<T> Serialize for Serializable<AmqpValue<T>>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde_amqp::serde::ser::SerializeTupleStruct;
        let mut state = serializer
            .serialize_tuple_struct(serde_amqp::__constants::DESCRIBED_BASIC, 1usize + 1)?;
        state.serialize_field(&serde_amqp::descriptor::Descriptor::Code(0x0000_0000_0000_0077_u64))?;
        state.serialize_field(&self.0.0)?;
        state.end()
    }
}

impl<T> Serialize for Serializable<&AmqpValue<T>>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde_amqp::serde::ser::SerializeTupleStruct;
        let mut state = serializer
            .serialize_tuple_struct(serde_amqp::__constants::DESCRIBED_BASIC, 1usize + 1)?;
        state.serialize_field(&serde_amqp::descriptor::Descriptor::Code(0x0000_0000_0000_0077_u64))?;
        state.serialize_field(&self.0.0)?;
        state.end()
    }
}

impl<'de, T> de::Deserialize<'de> for Deserializable<AmqpValue<T>>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<T> {
            _field0: std::marker::PhantomData<T>,
        }
        impl<T> Visitor<T> {
            fn new() -> Self {
                Self {
                    _field0: std::marker::PhantomData,
                }
            }
        }
        impl<'de, T> serde_amqp::serde::de::Visitor<'de> for Visitor<T>
        where
            T: serde_amqp::serde::de::Deserialize<'de>,
        {
            type Value = Deserializable<AmqpValue<T>>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("amqp:amqp-value:*")
            }
            fn visit_seq<A>(self, mut __seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde_amqp::serde::de::SeqAccess<'de>,
            {
                let __descriptor: serde_amqp::descriptor::Descriptor =
                    match __seq.next_element()? {
                        Some(val) => val,
                        None => {
                            return Err(serde_amqp::serde::de::Error::custom(
                                "Expecting descriptor",
                            ))
                        }
                    };
                match __descriptor {
                    serde_amqp::descriptor::Descriptor::Name(__symbol) => {
                        if __symbol.into_inner() != "amqp:amqp-value:*" {
                            return Err(serde_amqp::serde::de::Error::custom(
                                "Descriptor mismatch",
                            ));
                        }
                    }
                    serde_amqp::descriptor::Descriptor::Code(__c) => {
                        if __c != 0x0000_0000_0000_0077_u64 {
                            return Err(serde_amqp::serde::de::Error::custom(
                                "Descriptor mismatch",
                            ));
                        }
                    }
                }
                let field0: T = match __seq.next_element()? {
                    Some(val) => val,
                    None => {
                        return Err(serde_amqp::serde::de::Error::custom(
                            "Insufficient number of items",
                        ))
                    }
                };
                Ok(Deserializable(AmqpValue(field0)))
            }
        }
        deserializer.deserialize_tuple_struct(
            serde_amqp::__constants::DESCRIBED_BASIC,
            1usize + 1,
            Visitor::new(),
        )
    }
}
