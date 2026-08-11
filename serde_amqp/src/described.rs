//! Definition of `Described<T>` type

use std::marker::PhantomData;

use serde::{de, ser};

use crate::{
    __constants::{DESCRIBED_BASIC, DESCRIPTOR},
    descriptor::Descriptor,
    Value,
};

/// Contains a descriptor and a wrapped value T.
///
/// This should usually be avoided other than in Value type.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Described<T> {
    /// Descriptor of descriptor
    pub descriptor: Descriptor,

    /// Value of described
    pub value: T,
}

impl<T> Described<T>
where
    T: Into<Value>,
{
    /// Convert `Described<T>` to `Described<Value>`
    pub fn into_described_value(self) -> Described<Value> {
        let value: Value = self.value.into();
        Described {
            descriptor: self.descriptor,
            value,
        }
    }
}

impl<T: ser::Serialize> ser::Serialize for Described<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use ser::SerializeStruct;
        let mut state = serializer.serialize_struct(DESCRIBED_BASIC, 2)?;
        state.serialize_field(DESCRIPTOR, &self.descriptor)?;
        state.serialize_field("value", &self.value)?;
        state.end()
    }
}

struct Visitor<'de, T> {
    marker: PhantomData<T>,
    lifetime: PhantomData<&'de ()>,
}

impl<'de, T: de::Deserialize<'de>> de::Visitor<'de> for Visitor<'de, T> {
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
            None => return Err(de::Error::custom("Expecting descriptor")),
        };

        let value: T = match seq.next_element()? {
            Some(val) => val,
            None => {
                return Err(de::Error::custom(
                    "Insufficient number of elements. Expecting value",
                ))
            }
        };

        Ok(Described { descriptor, value })
    }
}

impl<'de, T: de::Deserialize<'de>> de::Deserialize<'de> for Described<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[DESCRIPTOR, "value"];
        deserializer.deserialize_struct(
            DESCRIBED_BASIC,
            FIELDS,
            Visitor {
                marker: PhantomData,
                lifetime: PhantomData,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::{descriptor::Descriptor, from_slice, to_vec};

    use super::Described;

    #[test]
    fn test_deserialize_described_value() {
        let descriptor = Descriptor::Code(0x11);
        let value = vec![1i32, 2];
        let described = Described { descriptor, value };
        let buf = to_vec(&described).unwrap();
        // DescribedType marker, smallulong descriptor 0x11, list8 with two smallint
        // elements
        assert_eq!(
            buf,
            vec![0x00, 0x53, 0x11, 0xC0, 0x05, 0x02, 0x54, 0x01, 0x54, 0x02]
        );
        let recovered: Described<Vec<i32>> = from_slice(&buf).unwrap();
        assert_eq!(recovered, described);
    }
}
