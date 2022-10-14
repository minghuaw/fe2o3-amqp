use serde_amqp::{
    macros::{DeserializeComposite, SerializeComposite},
    primitives::Boolean,
};

use crate::{
    definitions::{DeliveryNumber, Role},
    messaging::DeliveryState,
};

/// 2.7.6 Disposition
/// Inform remote peer of delivery state changes.
/// <type name="disposition" class="composite" source="list" provides="frame">
///     <descriptor name="amqp:disposition:list" code="0x00000000:0x00000015"/>
/// </type>
#[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
// #[serde(rename_all = "kebab-case")]
#[amqp_contract(
    name = "amqp:disposition:list",
    code = "0x0000_0000:0x0000_0015",
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Disposition {
    /// <field name="role" type="role" mandatory="true"/>
    pub role: Role,

    /// <field name="first" type="delivery-number" mandatory="true"/>
    pub first: DeliveryNumber,

    /// <field name="last" type="delivery-number"/>
    pub last: Option<DeliveryNumber>,

    /// <field name="settled" type="boolean" default="false"/>
    #[amqp_contract(default)]
    pub settled: Boolean,

    /// <field name="state" type="*" requires="delivery-state"/>
    pub state: Option<DeliveryState>,

    /// <field name="batchable" type="boolean" default="false"/>
    #[amqp_contract(default)]
    pub batchable: Boolean,
}

#[cfg(test)]
mod tests {
    use serde_amqp::{from_slice, primitives::Boolean, to_vec};

    use crate::{
        definitions::{DeliveryNumber, Role},
        messaging::{Accepted, DeliveryState},
    };

    #[test]
    fn test_serialize() {
        use super::Disposition;
        let disposition = Disposition {
            role: Role::Receiver,
            first: 0,
            last: None,
            settled: true,
            state: Some(DeliveryState::Accepted(Accepted {})),
            batchable: false,
        };
        let buf = to_vec(&disposition).unwrap();
        let expected = &[
            0x0u8, 0x53, 0x15, 0xc0, 0x9, 0x5, 0x41, 0x43, 0x40, 0x41, 0x0, 0x53, 0x24, 0x45,
        ];
        assert_eq!(buf, expected);
    }

    #[derive(Debug, Clone)]
    pub struct Disposition {
        /// <field name="role" type="role" mandatory="true"/>
        pub role: Role,

        /// <field name="first" type="delivery-number" mandatory="true"/>
        pub first: DeliveryNumber,

        /// <field name="last" type="delivery-number"/>
        pub last: Option<DeliveryNumber>,

        /// <field name="settled" type="boolean" default="false"/>
        pub settled: Boolean,

        /// <field name="state" type="*" requires="delivery-state"/>
        pub state: Option<DeliveryState>,

        /// <field name="batchable" type="boolean" default="false"/>
        pub batchable: Boolean,
    }

    const _: () = {
        #[automatically_derived]
        impl<'de> serde_amqp::serde::de::Deserialize<'de> for Disposition {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde_amqp::serde::de::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                enum Field {
                    role,
                    first,
                    last,
                    settled,
                    state,
                    batchable,
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
                            "role" => Ok(Self::Value::role),
                            "first" => Ok(Self::Value::first),
                            "last" => Ok(Self::Value::last),
                            "settled" => Ok(Self::Value::settled),
                            "state" => Ok(Self::Value::state),
                            "batchable" => Ok(Self::Value::batchable),
                            _ => Err(serde_amqp::serde::de::Error::custom("Unknown identifier")),
                        }
                    }
                    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                    where
                        E: serde_amqp::serde::de::Error,
                    {
                        match v {
                            b if b == "role".as_bytes() => Ok(Self::Value::role),
                            b if b == "first".as_bytes() => Ok(Self::Value::first),
                            b if b == "last".as_bytes() => Ok(Self::Value::last),
                            b if b == "settled".as_bytes() => Ok(Self::Value::settled),
                            b if b == "state".as_bytes() => Ok(Self::Value::state),
                            b if b == "batchable".as_bytes() => Ok(Self::Value::batchable),
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
                    type Value = Disposition;
                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("struct amqp:disposition:list")
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
                                if __symbol.into_inner() != "amqp:disposition:list" {
                                    return Err(serde_amqp::serde::de::Error::custom(
                                        "Descriptor mismatch",
                                    ));
                                }
                            }
                            serde_amqp::descriptor::Descriptor::Code(__c) => {
                                if __c != 21u64 {
                                    return Err(serde_amqp::serde::de::Error::custom(
                                        "Descriptor mismatch",
                                    ));
                                }
                            }
                        }
                        let role: Role = match __seq.next_element()? {
                            Some(val) => val,
                            None => {
                                return Err(serde_amqp::serde::de::Error::custom(
                                    "Insufficient number of items",
                                ))
                            }
                        };
                        let first: DeliveryNumber = match __seq.next_element()? {
                            Some(val) => val,
                            None => {
                                return Err(serde_amqp::serde::de::Error::custom(
                                    "Insufficient number of items",
                                ))
                            }
                        };
                        let last: Option<DeliveryNumber> = match __seq.next_element()? {
                            Some(val) => val,
                            None => None,
                        };
                        let settled: Boolean = match __seq.next_element()? {
                            Some(val) => val,
                            None => Default::default(),
                        };
                        let state: Option<DeliveryState> = match __seq.next_element()? {
                            Some(val) => val,
                            None => None,
                        };
                        let batchable: Boolean = match __seq.next_element()? {
                            Some(val) => val,
                            None => Default::default(),
                        };
                        Ok(Disposition {
                            role,
                            first,
                            last,
                            settled,
                            state,
                            batchable,
                        })
                    }
                    fn visit_map<A>(self, mut __map: A) -> Result<Self::Value, A::Error>
                    where
                        A: serde_amqp::serde::de::MapAccess<'de>,
                    {
                        let mut role: Option<Role> = None;
                        let mut first: Option<DeliveryNumber> = None;
                        let mut last: Option<Option<DeliveryNumber>> = None;
                        let mut settled: Option<Boolean> = None;
                        let mut state: Option<Option<DeliveryState>> = None;
                        let mut batchable: Option<Boolean> = None;
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
                                if __symbol.into_inner() != "amqp:disposition:list" {
                                    return Err(serde_amqp::serde::de::Error::custom(
                                        "Descriptor mismatch",
                                    ));
                                }
                            }
                            serde_amqp::descriptor::Descriptor::Code(__c) => {
                                if __c != 21u64 {
                                    return Err(serde_amqp::serde::de::Error::custom(
                                        "Descriptor mismatch",
                                    ));
                                }
                            }
                        }
                        while let Some(key) = __map.next_key::<Field>()? {
                            match key {
                                Field::role => {
                                    if role.is_some() {
                                        return Err(serde_amqp::serde::de::Error::duplicate_field(
                                            "role",
                                        ));
                                    }
                                    role = Some(__map.next_value()?);
                                }
                                Field::first => {
                                    if first.is_some() {
                                        return Err(serde_amqp::serde::de::Error::duplicate_field(
                                            "first",
                                        ));
                                    }
                                    first = Some(__map.next_value()?);
                                }
                                Field::last => {
                                    if last.is_some() {
                                        return Err(serde_amqp::serde::de::Error::duplicate_field(
                                            "last",
                                        ));
                                    }
                                    last = Some(__map.next_value()?);
                                }
                                Field::settled => {
                                    if settled.is_some() {
                                        return Err(serde_amqp::serde::de::Error::duplicate_field(
                                            "settled",
                                        ));
                                    }
                                    settled = Some(__map.next_value()?);
                                }
                                Field::state => {
                                    if state.is_some() {
                                        return Err(serde_amqp::serde::de::Error::duplicate_field(
                                            "state",
                                        ));
                                    }
                                    state = Some(__map.next_value()?);
                                }
                                Field::batchable => {
                                    if batchable.is_some() {
                                        return Err(serde_amqp::serde::de::Error::duplicate_field(
                                            "batchable",
                                        ));
                                    }
                                    batchable = Some(__map.next_value()?);
                                }
                            }
                        }
                        let role: Role = match role {
                            Some(val) => val,
                            None => {
                                return Err(serde_amqp::serde::de::Error::custom(
                                    "Insufficient number of items",
                                ))
                            }
                        };
                        let first: DeliveryNumber = match first {
                            Some(val) => val,
                            None => {
                                return Err(serde_amqp::serde::de::Error::custom(
                                    "Insufficient number of items",
                                ))
                            }
                        };
                        let last: Option<DeliveryNumber> = match last {
                            Some(val) => val,
                            None => None,
                        };
                        let settled: Boolean = match settled {
                            Some(val) => val,
                            None => Default::default(),
                        };
                        let state: Option<DeliveryState> = match state {
                            Some(val) => val,
                            None => None,
                        };
                        let batchable: Boolean = match batchable {
                            Some(val) => val,
                            None => Default::default(),
                        };
                        Ok(Disposition {
                            role,
                            first,
                            last,
                            settled,
                            state,
                            batchable,
                        })
                    }
                }
                const FIELDS: &[&str] = &[
                    serde_amqp::__constants::DESCRIPTOR,
                    "role",
                    "first",
                    "last",
                    "settled",
                    "state",
                    "batchable",
                ];
                deserializer.deserialize_struct(
                    serde_amqp::__constants::DESCRIBED_LIST,
                    FIELDS,
                    Visitor {},
                )
            }
        }
    };

    #[test]
    fn test_deserialize() {
        let buf = &[
            0x0u8, 0x53, 0x15, 0xc0, 0x9, 0x5, 0x41, 0x43, 0x40, 0x41, 0x0, 0x53, 0x24, 0x45,
        ];
        let deserialized: Disposition = from_slice(&buf[..]).unwrap();
        println!("{:?}", deserialized);
    }
}
