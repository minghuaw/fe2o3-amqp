use std::fmt::Debug;

use serde::de;
use serde::ser;

use super::{Descriptor, DESCRIPTOR};
use crate::format_code::EncodingCodes;

pub const DESCRIBED_BASIC: &str = "DESCRIBED_BASIC";
pub const DESCRIBED_LIST: &str = "DESCRIBED_LIST";
pub const DESCRIBED_MAP: &str = "DESCRIBED_MAP";
pub const DESERIALIZE_DESCRIBED: &str = "DESERIALIZE_DESCRIBED";

pub const DESCRIBED_FIELDS: &'static [&'static str] = &["descriptor", "encoding_type", "value"];

pub const ENCODING_TYPE: &str = "ENCODING_TYPE";

#[derive(Debug)]
pub enum EncodingType {
    Basic,
    List,
    Map,
}

mod encoding_type {
    use std::convert::TryInto;

    use super::*;

    enum Field {
        Basic,
        List,
        Map,
    }

    struct FieldVisitor {}

    impl<'de> de::Visitor<'de> for FieldVisitor {
        type Value = Field;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("variant identifier")
        }

        fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match v
                .try_into()
                .map_err(|_| de::Error::custom("Invalid format code"))?
            {
                EncodingCodes::List0 | EncodingCodes::List32 | EncodingCodes::List8 => {
                    Ok(Field::List)
                }
                EncodingCodes::Map8 | EncodingCodes::Map32 => Ok(Field::Map),
                _ => Ok(Field::Basic),
            }
        }
    }

    impl<'de> de::Deserialize<'de> for Field {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_ignored_any(FieldVisitor {})
        }
    }

    struct EncodingTypeVisitor {}

    impl<'de> de::Visitor<'de> for EncodingTypeVisitor {
        type Value = EncodingType;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("enum EncodingType")
        }

        fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
        where
            A: de::EnumAccess<'de>,
        {
            let (val, _) = data.variant()?;
            match val {
                Field::Basic => Ok(EncodingType::Basic),
                Field::List => Ok(EncodingType::List),
                Field::Map => Ok(EncodingType::Map),
            }
        }
    }

    impl<'de> de::Deserialize<'de> for EncodingType {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            const VARIANTS: &'static [&'static str] = &["Basic", "List", "Map"];
            deserializer.deserialize_enum(ENCODING_TYPE, VARIANTS, EncodingTypeVisitor {})
        }
    }
}

/// The described type will attach a descriptor before the value.
/// There is no generic implementation of serialization. But a inner type
/// specific implementation will be generated via macro.
pub struct Described<T> {
    descriptor: Descriptor,
    encoding_type: EncodingType,
    value: T,
}

impl<T> Described<T> {
    pub fn new(encoding: EncodingType, descriptor: Descriptor, value: T) -> Self {
        Self {
            descriptor,
            encoding_type: encoding,
            value,
        }
    }
}

impl<T: Debug> Debug for Described<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Described")
            .field("descriptor", &self.descriptor)
            .field("encoding_type", &self.encoding_type)
            .field("value", &self.value)
            .finish()
    }
}

impl<'a, T: ser::Serialize> ser::Serialize for Described<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        use serde::ser::SerializeStruct;

        let name = match self.encoding_type {
            EncodingType::Basic => DESCRIBED_BASIC,
            EncodingType::List => DESCRIBED_LIST,
            EncodingType::Map => DESCRIBED_MAP,
        };
        let mut state = serializer.serialize_struct(name, 2)?;
        state.serialize_field(DESCRIPTOR, &self.descriptor)?;
        state.serialize_field("value", &self.value)?;
        state.end()
    }
}

mod described {
    use super::*;
    use std::marker::PhantomData;

    struct DescribedVisitor<'de, T> {
        marker: PhantomData<T>,
        lifetime: PhantomData<&'de ()>,
    }

    impl<'de, T: de::Deserialize<'de>> de::Visitor<'de> for DescribedVisitor<'de, T> {
        type Value = Described<T>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("struct Described")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let descriptor: Descriptor = match seq.next_element()? {
                Some(val) => val,
                None => return Err(de::Error::custom("Invalid length. Expecting descriptor.")),
            };

            let encoding_type: EncodingType = match seq.next_element()? {
                Some(val) => val,
                None => return Err(de::Error::custom("Invalid length. Expecting encoding_type")),
            };

            let value: T = match seq.next_element()? {
                Some(val) => val,
                None => return Err(de::Error::custom("Invalid length. Expecting value")),
            };

            Ok(Described {
                descriptor,
                encoding_type,
                value,
            })
        }
    }

    impl<'de, T> de::Deserialize<'de> for Described<T>
    where
        T: de::Deserialize<'de>,
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_struct(
                DESERIALIZE_DESCRIBED,
                DESCRIBED_FIELDS,
                DescribedVisitor {
                    marker: PhantomData,
                    lifetime: PhantomData,
                },
            )
        }
    }
}

// implement conversion to described type for the fundamental types

macro_rules! impl_err_try_conversion_to_described {
    ($($t:ty), *) => {
        $(
            impl std::convert::TryFrom<$t> for Described<$t> {
                type Error = $t;

                fn try_from(value: $t) -> Result<Self, Self::Error> {
                    Err(value)
                }
            }
        )*
    };
}

impl_err_try_conversion_to_described!(
    u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64,
    bool // TODO: add more fundamental types
);
