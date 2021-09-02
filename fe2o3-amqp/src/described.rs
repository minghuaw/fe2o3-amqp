use std::marker::PhantomData;

use serde::{ser, de};

use crate::{constants::{DESCRIBED_BASIC, DESCRIPTOR}, descriptor::Descriptor};

/// Contains a Box to descriptor and a Box to value T.
///
/// This should usually be avoided other than in Value type. 
/// Two pointers are used to reduce the memory size of the Value type.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Described<T> {
    descriptor: Box<Descriptor>,
    value: Box<T>
}

impl<T: ser::Serialize> ser::Serialize for Described<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
            S: serde::Serializer {
        use ser::SerializeStruct;
        let mut state = serializer.serialize_struct(DESCRIBED_BASIC, 2)?;
        state.serialize_field(DESCRIPTOR, &self.descriptor)?;
        state.serialize_field("value", &self.value)?;
        state.end()
    }
}

struct Visitor<'de, T> { 
    marker: PhantomData<T>,
    lifetime: PhantomData<&'de ()>
}

impl<'de, T: de::Deserialize<'de>> de::Visitor<'de> for Visitor<'de, T> {
    type Value = Described<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("struct Described")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
            A: de::SeqAccess<'de>, {
        let descriptor: Descriptor = match seq.next_element()? {
            Some(val) => val,
            None => {
                return Err(de::Error::custom(
                    "Expecting descriptor",
                ))
            }
        };

        let value: T = match seq.next_element()? {
            Some(val) => val,
            None => {
                return Err(de::Error::custom(
                    "Insufficient number of elements. Expecting value",
                ))
            }
        };

        Ok(Described { descriptor: Box::new(descriptor), value: Box::new(value) })
    }
}


impl<'de, T: de::Deserialize<'de>> de::Deserialize<'de> for Described<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
            D: serde::Deserializer<'de> {
        const FIELDS: &'static [&'static str] = &[DESCRIPTOR, "value"];
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
    use crate::{from_slice, to_vec, descriptor::Descriptor};

    use super::Described;

    #[test]
    fn test_serialize_described_value() {
        let descriptor = Box::new(Descriptor::Code(0x11));
        let value = Box::new(vec![1i32, 2]);
        let described = Described {
            descriptor,
            value
        };
        let buf = to_vec(&described).unwrap();
        println!("{:x?}", buf);
    }

    #[test]
    fn test_deserialzie_described_value() {
        let descriptor = Box::new(Descriptor::Code(0x11));
        let value = Box::new(vec![1i32, 2]);
        let described = Described {
            descriptor,
            value
        };
        let buf = to_vec(&described).unwrap();
        let recovered: Described<Vec<i32>> = from_slice(&buf).unwrap();
        println!("{:?}", recovered);
    }
}
