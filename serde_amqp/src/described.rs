use std::marker::PhantomData;

use serde::{de, ser};

use crate::{
    __constants::{DESCRIBED_BASIC, DESCRIPTOR},
    descriptor::Descriptor,
};

/// Contains a Box to descriptor and a Box to value T.
///
/// This should usually be avoided other than in Value type.
/// Two pointers are used to reduce the memory size of the Value type.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Described<T> {
    pub descriptor: Box<Descriptor>,
    pub value: Box<T>,
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

        Ok(Described {
            descriptor: Box::new(descriptor),
            value: Box::new(value),
        })
    }
}

impl<'de, T: de::Deserialize<'de>> de::Deserialize<'de> for Described<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
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
    use crate::{descriptor::Descriptor, from_slice, to_vec};

    use super::Described;

    #[test]
    fn test_serialize_described_value() {
        let descriptor = Box::new(Descriptor::Code(0x11));
        let value = Box::new(vec![1i32, 2]);
        let described = Described { descriptor, value };
        let buf = to_vec(&described).unwrap();
        println!("{:x?}", buf);
    }

    #[test]
    fn test_deserialzie_described_value() {
        let descriptor = Box::new(Descriptor::Code(0x11));
        let value = Box::new(vec![1i32, 2]);
        let described = Described { descriptor, value };
        let buf = to_vec(&described).unwrap();
        println!("{:?}", &buf);
        let recovered: Described<Vec<i32>> = from_slice(&buf).unwrap();
        println!("{:?}", recovered);
    }

    // Expanded macro
    use crate as serde_amqp;

    #[derive(Debug, PartialEq)]
    struct Foo {
        b: u64,
        is_fool: Option<bool>,
        a: i32,
    }
    const _: () = {
        #[automatically_derived]
        impl serde_amqp::serde::ser::Serialize for Foo {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde_amqp::serde::ser::Serializer,
            {
                use serde_amqp::serde::ser::SerializeStruct;
                let mut nulls: Vec<&str> = Vec::new();
                let mut state = serializer
                    .serialize_struct(serde_amqp::__constants::DESCRIBED_LIST, 3usize + 1)?;
                state.serialize_field(
                    serde_amqp::__constants::DESCRIPTOR,
                    &serde_amqp::descriptor::Descriptor::Code(19u64),
                )?;
                for field_name in nulls.drain(..) {
                    state.serialize_field(field_name, &())?;
                }
                state.serialize_field("b", &self.b)?;
                if (&self.is_fool).is_some() {
                    for field_name in nulls.drain(..) {
                        state.serialize_field(field_name, &())?;
                    }
                    state.serialize_field("is-fool", &self.is_fool)?;
                } else {
                    nulls.push("is-fool");
                };
                if *&self.a != <i32 as Default>::default() {
                    for field_name in nulls.drain(..) {
                        state.serialize_field(field_name, &())?;
                    }
                    state.serialize_field("a", &self.a)?;
                } else {
                    nulls.push("a");
                };
                state.end()
            }
        }
    };
    const _: () = {
        #[automatically_derived]
        impl<'de> serde_amqp::serde::de::Deserialize<'de> for Foo {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde_amqp::serde::de::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                enum Field {
                    b,
                    is_fool,
                    a,
                }
                struct FieldVisitor {}
                impl<'de> serde_amqp::serde::de::Visitor<'de> for FieldVisitor {
                    type Value = Field;
                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("field identifier")
                    }
                    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                    where
                        E: serde_amqp::serde::de::Error,
                    {
                        match v {
                            "b" => Ok(Self::Value::b),
                            "is-fool" => Ok(Self::Value::is_fool),
                            "a" => Ok(Self::Value::a),
                            _ => Err(serde_amqp::serde::de::Error::custom("Unknown identifier")),
                        }
                    }
                    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                    where
                        E: serde_amqp::serde::de::Error,
                    {
                        match v {
                            b if b == "b".as_bytes() => Ok(Self::Value::b),
                            b if b == "is-fool".as_bytes() => Ok(Self::Value::is_fool),
                            b if b == "a".as_bytes() => Ok(Self::Value::a),
                            _ => Err(serde_amqp::serde::de::Error::custom("Unknown identifier")),
                        }
                    }
                }
                impl<'de> serde_amqp::serde::de::Deserialize<'de> for Field {
                    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                    where
                        D: serde_amqp::serde::de::Deserializer<'de>,
                    {
                        deserializer.deserialize_identifier(FieldVisitor {})
                    }
                }
                struct Visitor {}
                impl<'de> serde_amqp::serde::de::Visitor<'de> for Visitor {
                    type Value = Foo;
                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("struct Foo")
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
                                if __symbol.into_inner() != "Foo" {
                                    return Err(serde_amqp::serde::de::Error::custom(
                                        "Descriptor mismatch",
                                    ));
                                }
                            }
                            serde_amqp::descriptor::Descriptor::Code(__c) => {
                                if __c != 19u64 {
                                    return Err(serde_amqp::serde::de::Error::custom(
                                        "Descriptor mismatch",
                                    ));
                                }
                            }
                        }
                        let b: u64 = match __seq.next_element()? {
                            Some(val) => val,
                            None => {
                                return Err(serde_amqp::serde::de::Error::custom(
                                    "Insufficient number of items",
                                ))
                            }
                        };
                        let is_fool: Option<bool> = match __seq.next_element()? {
                            Some(val) => val,
                            None => None,
                        };
                        let a: i32 = match __seq.next_element()? {
                            Some(val) => val,
                            None => Default::default(),
                        };
                        Ok(Foo { b, is_fool, a })
                    }
                    fn visit_map<A>(self, mut __map: A) -> Result<Self::Value, A::Error>
                    where
                        A: serde_amqp::serde::de::MapAccess<'de>,
                    {
                        let mut b: Option<u64> = None;
                        let mut is_fool: Option<Option<bool>> = None;
                        let mut a: Option<i32> = None;
                        let __descriptor: serde_amqp::descriptor::Descriptor =
                            match __map.next_key()? {
                                Some(val) => val,
                                None => {
                                    return Err(serde_amqp::serde::de::Error::custom(
                                        "Expecting__descriptor",
                                    ))
                                }
                            };
                        match __descriptor {
                            serde_amqp::descriptor::Descriptor::Name(__symbol) => {
                                if __symbol.into_inner() != "Foo" {
                                    return Err(serde_amqp::serde::de::Error::custom(
                                        "Descriptor mismatch",
                                    ));
                                }
                            }
                            serde_amqp::descriptor::Descriptor::Code(__c) => {
                                if __c != 19u64 {
                                    return Err(serde_amqp::serde::de::Error::custom(
                                        "Descriptor mismatch",
                                    ));
                                }
                            }
                        }
                        while let Some(key) = __map.next_key::<Field>()? {
                            match key {
                                Field::b => {
                                    if b.is_some() {
                                        return Err(serde_amqp::serde::de::Error::duplicate_field(
                                            "b",
                                        ));
                                    }
                                    b = Some(__map.next_value()?);
                                }
                                Field::is_fool => {
                                    if is_fool.is_some() {
                                        return Err(serde_amqp::serde::de::Error::duplicate_field(
                                            "is-fool",
                                        ));
                                    }
                                    is_fool = Some(__map.next_value()?);
                                }
                                Field::a => {
                                    if a.is_some() {
                                        return Err(serde_amqp::serde::de::Error::duplicate_field(
                                            "a",
                                        ));
                                    }
                                    a = Some(__map.next_value()?);
                                }
                            }
                        }
                        let b: u64 = match b {
                            Some(val) => val,
                            None => {
                                return Err(serde_amqp::serde::de::Error::custom(
                                    "Insufficient number of items",
                                ))
                            }
                        };
                        let is_fool: Option<bool> = match is_fool {
                            Some(val) => val,
                            None => None,
                        };
                        let a: i32 = match a {
                            Some(val) => val,
                            None => Default::default(),
                        };
                        Ok(Foo { b, is_fool, a })
                    }
                }
                const FIELDS: &'static [&'static str] =
                    &[serde_amqp::__constants::DESCRIPTOR, "b", "is-fool", "a"];
                deserializer.deserialize_struct(
                    serde_amqp::__constants::DESCRIBED_LIST,
                    FIELDS,
                    Visitor {},
                )
            }
        }
    };

    #[test]
    fn test_expanded_list_macro() {
        let foo = Foo {
            b: 0,
            is_fool: None,
            a: 0,
        };
        let serialized = to_vec(&foo).unwrap();
        println!("{:?}", &serialized);
        let deserialized: Foo = from_slice(&serialized).unwrap();
        println!("{:?}", &deserialized);
        assert_eq!(foo, deserialized);
    }

    #[derive(PartialEq)]
    struct Test {
        a: Option<i32>,
        b: bool,
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for Test {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                Test {
                    a: ref __self_0_0,
                    b: ref __self_0_1,
                } => {
                    let debug_trait_builder = &mut ::core::fmt::Formatter::debug_struct(f, "Test");
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder, "a", &&(*__self_0_0));
                    let _ =
                        ::core::fmt::DebugStruct::field(debug_trait_builder, "b", &&(*__self_0_1));
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    const _: () = {
        #[automatically_derived]
        #[allow(unused_mut)]
        impl serde_amqp::serde::ser::Serialize for Test {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde_amqp::serde::ser::Serializer,
            {
                use serde_amqp::serde::ser::SerializeStruct;
                let mut nulls: Vec<&str> = Vec::new();
                let mut state = serializer
                    .serialize_struct(serde_amqp::__constants::DESCRIBED_MAP, 2usize + 1)?;
                state.serialize_field(
                    serde_amqp::__constants::DESCRIPTOR,
                    &serde_amqp::descriptor::Descriptor::Name(
                        serde_amqp::primitives::Symbol::from("ab"),
                    ),
                )?;
                if (&self.a).is_some() {
                    state.serialize_field("a", &self.a)?;
                };
                if *&self.b != <bool as Default>::default() {
                    state.serialize_field("b", &self.b)?;
                };
                state.end()
            }
        }
    };
    const _: () = {
        #[automatically_derived]
        impl<'de> serde_amqp::serde::de::Deserialize<'de> for Test {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde_amqp::serde::de::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                enum Field {
                    a,
                    b,
                }
                struct FieldVisitor {}
                impl<'de> serde_amqp::serde::de::Visitor<'de> for FieldVisitor {
                    type Value = Field;
                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("field identifier")
                    }
                    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                    where
                        E: serde_amqp::serde::de::Error,
                    {
                        match v {
                            "a" => Ok(Self::Value::a),
                            "b" => Ok(Self::Value::b),
                            _ => Err(serde_amqp::serde::de::Error::custom("Unknown identifier")),
                        }
                    }
                    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                    where
                        E: serde_amqp::serde::de::Error,
                    {
                        match v {
                            b if b == "a".as_bytes() => Ok(Self::Value::a),
                            b if b == "b".as_bytes() => Ok(Self::Value::b),
                            _ => Err(serde_amqp::serde::de::Error::custom("Unknown identifier")),
                        }
                    }
                }
                impl<'de> serde_amqp::serde::de::Deserialize<'de> for Field {
                    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                    where
                        D: serde_amqp::serde::de::Deserializer<'de>,
                    {
                        deserializer.deserialize_identifier(FieldVisitor {})
                    }
                }
                struct Visitor {}
                impl<'de> serde_amqp::serde::de::Visitor<'de> for Visitor {
                    type Value = Test;
                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("struct ab")
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
                                if __symbol.into_inner() != "ab" {
                                    return Err(serde_amqp::serde::de::Error::custom(
                                        "Descriptor mismatch",
                                    ));
                                }
                            }
                            serde_amqp::descriptor::Descriptor::Code(_) => {
                                return Err(serde_amqp::serde::de::Error::custom(
                                    "Descriptor mismatch",
                                ))
                            }
                        }
                        let a: Option<i32> = match __seq.next_element()? {
                            Some(val) => val,
                            None => None,
                        };
                        let b: bool = match __seq.next_element()? {
                            Some(val) => val,
                            None => Default::default(),
                        };
                        Ok(Test { a, b })
                    }
                    fn visit_map<A>(self, mut __map: A) -> Result<Self::Value, A::Error>
                    where
                        A: serde_amqp::serde::de::MapAccess<'de>,
                    {
                        let mut a: Option<Option<i32>> = None;
                        let mut b: Option<bool> = None;
                        let __descriptor: serde_amqp::descriptor::Descriptor =
                            match __map.next_key()? {
                                Some(val) => val,
                                None => {
                                    return Err(serde_amqp::serde::de::Error::custom(
                                        "Expecting__descriptor",
                                    ))
                                }
                            };
                        match __descriptor {
                            serde_amqp::descriptor::Descriptor::Name(__symbol) => {
                                if __symbol.into_inner() != "ab" {
                                    return Err(serde_amqp::serde::de::Error::custom(
                                        "Descriptor mismatch",
                                    ));
                                }
                            }
                            serde_amqp::descriptor::Descriptor::Code(_) => {
                                return Err(serde_amqp::serde::de::Error::custom(
                                    "Descriptor mismatch",
                                ))
                            }
                        }
                        while let Some(key) = __map.next_key::<Field>()? {
                            match key {
                                Field::a => {
                                    if a.is_some() {
                                        return Err(serde_amqp::serde::de::Error::duplicate_field(
                                            "a",
                                        ));
                                    }
                                    a = Some(__map.next_value()?);
                                }
                                Field::b => {
                                    if b.is_some() {
                                        return Err(serde_amqp::serde::de::Error::duplicate_field(
                                            "b",
                                        ));
                                    }
                                    b = Some(__map.next_value()?);
                                }
                            }
                        }
                        let a: Option<i32> = match a {
                            Some(val) => val,
                            None => None,
                        };
                        let b: bool = match b {
                            Some(val) => val,
                            None => Default::default(),
                        };
                        Ok(Test { a, b })
                    }
                }
                const FIELDS: &'static [&'static str] =
                    &[serde_amqp::__constants::DESCRIPTOR, "a", "b"];
                deserializer.deserialize_struct(
                    serde_amqp::__constants::DESCRIBED_MAP,
                    FIELDS,
                    Visitor {},
                )
            }
        }
    };

    #[test]
    fn test_expanded_map_macro() {
        let test = Test {
            a: Some(1),
            b: true,
        };
        let serialized = to_vec(&test).unwrap();
        println!("{:?}", &serialized);
        let deserialized: Test = from_slice(&serialized).unwrap();
        println!("{:?}", &deserialized);
        assert_eq!(test, deserialized);
    }
}
