//! Manually implement Serialize and Deserialize for SaslMechanisms

use serde::{de, ser};
use serde_amqp::primitives::{Array, Symbol};

use super::SaslMechanisms;

/// Entry in SaslMechanisms that represents a SASL Anonymous mechanism
pub const ANONYMOUS: &str = "ANONYMOUS";

impl Default for SaslMechanisms {
    /// Creates a new instance of SaslMechanisms
    ///
    /// A SASL mechanism ANONYMOUS is included by default
    ///
    /// It is invalid for this list to be null or empty. If the sending peer does not require
    /// its partner to authenticate with it, then it SHOULD send a list of one element with
    /// its value as the SASL mechanism ANONYMOUS.
    fn default() -> Self {
        Self {
            sasl_server_mechanisms: Array::from(vec![Symbol::from(ANONYMOUS)]),
        }
    }
}

impl serde_amqp::serde::ser::Serialize for SaslMechanisms {
    fn serialize<_S>(&self, serializer: _S) -> Result<_S::Ok, _S::Error>
    where
        _S: serde_amqp::serde::ser::Serializer,
    {
        use serde_amqp::serde::ser::SerializeStruct;

        // NOTE: A field which is defined as both multiple and mandatory MUST contain at least one value
        // (i.e. for such a field both null and an array with no entries are invalid).
        if self.sasl_server_mechanisms.0.is_empty() {
            return Err(ser::Error::custom(
                "A field which is defined as both multiple and mandatory MUST contain at least one value"
            ));
        }

        let mut state =
            serializer.serialize_struct(serde_amqp::__constants::DESCRIBED_LIST, 1usize + 1)?;
        state.serialize_field(
            serde_amqp::__constants::DESCRIPTOR,
            &serde_amqp::descriptor::Descriptor::Code(64u64),
        )?;
        state.serialize_field("sasl-server-mechanisms", &self.sasl_server_mechanisms)?;
        state.end()
    }
}

impl<'de> serde_amqp::serde::de::Deserialize<'de> for SaslMechanisms {
    fn deserialize<_D>(deserializer: _D) -> Result<Self, _D::Error>
    where
        _D: serde_amqp::serde::de::Deserializer<'de>,
    {
        #[allow(non_camel_case_types)]
        enum Field {
            sasl_server_mechanisms,
        }
        struct FieldVisitor {}
        impl<'de> serde_amqp::serde::de::Visitor<'de> for FieldVisitor {
            type Value = Field;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("field identifier")
            }
            fn visit_str<_E>(self, v: &str) -> Result<Self::Value, _E>
            where
                _E: serde_amqp::serde::de::Error,
            {
                match v {
                    "sasl-server-mechanisms" => Ok(Self::Value::sasl_server_mechanisms),
                    _ => Err(serde_amqp::serde::de::Error::custom("Unknown identifier")),
                }
            }
            fn visit_bytes<_E>(self, v: &[u8]) -> Result<Self::Value, _E>
            where
                _E: serde_amqp::serde::de::Error,
            {
                match v {
                    b if b == "sasl-server-mechanisms".as_bytes() => {
                        Ok(Self::Value::sasl_server_mechanisms)
                    }
                    _ => Err(serde_amqp::serde::de::Error::custom("Unknown identifier")),
                }
            }
        }
        impl<'de> serde_amqp::serde::de::Deserialize<'de> for Field {
            fn deserialize<_D>(deserializer: _D) -> Result<Self, _D::Error>
            where
                _D: serde_amqp::serde::de::Deserializer<'de>,
            {
                deserializer.deserialize_identifier(FieldVisitor {})
            }
        }
        struct Visitor {}
        impl Visitor {
            fn new() -> Self {
                Self {}
            }
        }
        impl<'de> serde_amqp::serde::de::Visitor<'de> for Visitor {
            type Value = SaslMechanisms;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct amqp:sasl-mechanisms:list")
            }
            fn visit_seq<_A>(self, mut __seq: _A) -> Result<Self::Value, _A::Error>
            where
                _A: serde_amqp::serde::de::SeqAccess<'de>,
            {
                let __descriptor: serde_amqp::descriptor::Descriptor = match __seq.next_element()? {
                    Some(val) => val,
                    None => {
                        return Err(serde_amqp::serde::de::Error::custom("Expecting descriptor"))
                    }
                };
                match __descriptor {
                    serde_amqp::descriptor::Descriptor::Name(__symbol) => {
                        if __symbol.into_inner() != "amqp:sasl-mechanisms:list" {
                            return Err(serde_amqp::serde::de::Error::custom(
                                "Descriptor mismatch",
                            ));
                        }
                    }
                    serde_amqp::descriptor::Descriptor::Code(__c) => {
                        if __c != 64u64 {
                            return Err(serde_amqp::serde::de::Error::custom(
                                "Descriptor mismatch",
                            ));
                        }
                    }
                }
                let sasl_server_mechanisms: Array<Symbol> = match __seq.next_element()? {
                    Some(val) => val,
                    None => {
                        return Err(serde_amqp::serde::de::Error::custom(
                            "Insufficient number of items",
                        ))
                    }
                };
                Ok(SaslMechanisms {
                    sasl_server_mechanisms,
                })
            }
            fn visit_map<_A>(self, mut __map: _A) -> Result<Self::Value, _A::Error>
            where
                _A: serde_amqp::serde::de::MapAccess<'de>,
            {
                let mut sasl_server_mechanisms: Option<Array<Symbol>> = None;
                let __descriptor: serde_amqp::descriptor::Descriptor = match __map.next_key()? {
                    Some(val) => val,
                    None => {
                        return Err(serde_amqp::serde::de::Error::custom(
                            "Expecting__descriptor",
                        ))
                    }
                };
                match __descriptor {
                    serde_amqp::descriptor::Descriptor::Name(__symbol) => {
                        if __symbol.into_inner() != "amqp:sasl-mechanisms:list" {
                            return Err(serde_amqp::serde::de::Error::custom(
                                "Descriptor mismatch",
                            ));
                        }
                    }
                    serde_amqp::descriptor::Descriptor::Code(__c) => {
                        if __c != 64u64 {
                            return Err(serde_amqp::serde::de::Error::custom(
                                "Descriptor mismatch",
                            ));
                        }
                    }
                }
                while let Some(key) = __map.next_key::<Field>()? {
                    match key {
                        Field::sasl_server_mechanisms => {
                            if sasl_server_mechanisms.is_some() {
                                return Err(serde_amqp::serde::de::Error::duplicate_field(
                                    "sasl-server-mechanisms",
                                ));
                            }
                            sasl_server_mechanisms = Some(__map.next_value()?);
                        }
                    }
                }
                let sasl_server_mechanisms: Array<Symbol> = match sasl_server_mechanisms {
                    Some(val) => val,
                    None => {
                        return Err(serde_amqp::serde::de::Error::custom(
                            "Insufficient number of items",
                        ))
                    }
                };
                Ok(SaslMechanisms {
                    sasl_server_mechanisms,
                })
            }
        }
        const FIELDS: &[&str] = &[
            serde_amqp::__constants::DESCRIPTOR,
            "sasl-server-mechanisms",
        ];
        let mechanisms = deserializer.deserialize_struct(
            serde_amqp::__constants::DESCRIBED_LIST,
            FIELDS,
            Visitor::new(),
        )?;

        if mechanisms.sasl_server_mechanisms.0.is_empty() {
            return Err(de::Error::custom(
                "A field which is defined as both multiple and mandatory MUST contain at least one value"
            ));
        }
        Ok(mechanisms)
    }
}
