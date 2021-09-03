use fe2o3_amqp::macros::{DeserializeComposite, SerializeComposite};
use fe2o3_amqp::primitives::{Boolean, Uint, Ulong};
use serde::{Deserialize, Serialize};

use crate::definitions::{Error, Fields};

/// 3.4 Delivery State
#[derive(Debug, Clone)]
pub enum DeliveryState {
    Accepted(Accepted),
    Rejected(Rejected),
    Released(Released),
    Modified(Modified),
    Received(Received),
}

mod delivery_state_serde {
    use serde::{de::{self, VariantAccess}, ser};

    use super::DeliveryState;

    impl ser::Serialize for DeliveryState {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer 
        {
            match self {
                DeliveryState::Accepted(value) => value.serialize(serializer),
                DeliveryState::Rejected(value) => value.serialize(serializer),
                DeliveryState::Released(value) => value.serialize(serializer),
                DeliveryState::Modified(value) => value.serialize(serializer),
                DeliveryState::Received(value) => value.serialize(serializer)
            }        
        }
    }

    enum Field {
        Accepted,
        Rejected,
        Released,
        Modified,
        Received
    }

    struct FieldVisitor {}

    impl<'de> de::Visitor<'de> for FieldVisitor {
        type Value = Field;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("variant identifier")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error, 
        {
            let val = match v {
                "amqp:accepted:list" => Field::Accepted,
                "amqp:rejected:list" => Field::Rejected,
                "amqp:released:list" => Field::Released,
                "amqp:modified:list" => Field::Modified,
                "amqp:received:list" => Field::Received,
                _ => return Err(de::Error::custom("Wrong symbol value for descriptor"))
            };

            Ok(val)
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
                E: de::Error, {
            let val = match v {
                0x0000_0000_0000_0023 => Field::Received,
                0x0000_0000_0000_0024 => Field::Accepted,
                0x0000_0000_0000_0025 => Field::Rejected,
                0x000_0000_0000_0026 => Field::Released,
                0x0000_0000_0000_0027 => Field::Modified,
                _ => return Err(de::Error::custom("Wrong code value for descriptor"))
            };
            Ok(val)
        }
    }

    impl<'de> de::Deserialize<'de> for Field {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
                D: serde::Deserializer<'de> {
            deserializer.deserialize_identifier(FieldVisitor { })
        }
    }

    struct Visitor { }

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = DeliveryState;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("enum DeliveryState")
        }

        fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
        where
                A: de::EnumAccess<'de>, {
            let (val, variant) = data.variant()?;

            match val {
                Field::Accepted => {
                    let value = variant.newtype_variant()?;
                    Ok(DeliveryState::Accepted(value))
                },
                Field::Rejected => {
                    let value = variant.newtype_variant()?;
                    Ok(DeliveryState::Rejected(value))
                }
                Field::Released => {
                    let value = variant.newtype_variant()?;
                    Ok(DeliveryState::Released(value))
                }
                Field::Modified => {
                    let value = variant.newtype_variant()?;
                    Ok(DeliveryState::Modified(value))
                },
                Field::Received => {
                    let value = variant.newtype_variant()?;
                    Ok(DeliveryState::Received(value))
                }
            }
        }
    }

    impl<'de> de::Deserialize<'de> for DeliveryState {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
                D: serde::Deserializer<'de> {
            const VARIANTS: &'static [&'static str] = &[
                "amqp:accepted:list",
                "amqp:rejected:list",
                "amqp:released:list",
                "amqp:modified:list",
                "amqp:received:list",
            ];
            deserializer.deserialize_enum("DeliveryState", VARIANTS, Visitor { })
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Outcome {
    Accepted(Accepted),
    Rejected(Rejected),
    Released(Released),
    Modified(Modified),
}

/// 3.4.1 Received
///
/// <type name="received" class="composite" source="list" provides="delivery-state">
/// <descriptor name="amqp:received:list" code="0x00000000:0x00000023"/>
/// <field name="section-number" type="uint" mandatory="true"/>
/// <field name="section-offset" type="ulong" mandatory="true"/>
/// </type>
#[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:received:list",
    code = 0x0000_0000_0000_0023,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Received {
    pub section_number: Uint,
    pub section_offset: Ulong,
}

/// 3.4.2 Accepted
/// The accepted outcome
///
/// <type name="accepted" class="composite" source="list" provides="delivery-state, outcome">
///     <descriptor name="amqp:accepted:list" code="0x00000000:0x00000024"/>
/// </type>
#[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:accepted:list",
    code = 0x0000_0000_0000_0024,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Accepted {}

/// 3.4.3 Rejected
/// The rejected outcome.
///
/// <type name="rejected" class="composite" source="list" provides="delivery-state, outcome">
///     <descriptor name="amqp:rejected:list" code="0x00000000:0x00000025"/>
///     <field name="error" type="error"/>
/// </type>
#[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:rejected:list",
    code = 0x0000_0000_0000_0025,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Rejected {
    pub error: Option<Error>,
}

/// 3.4.4 Released
/// The released outcome.
/// <type name="released" class="composite" source="list" provides="delivery-state, outcome">
///     <descriptor name="amqp:released:list" code="0x00000000:0x00000026"/>
/// </type>
#[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:released:list",
    code = 0x000_0000_0000_0026,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Released {}

/// 3.4.5 Modified
/// The modified outcome.
/// <type name="modified" class="composite" source="list" provides="delivery-state, outcome">
///     <descriptor name="amqp:modified:list" code="0x00000000:0x00000027"/>
///     <field name="delivery-failed" type="boolean"/>
///     <field name="undeliverable-here" type="boolean"/>
///     <field name="message-annotations" type="fields"/>
/// </type>
#[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:modified:list",
    code = 0x0000_0000_0000_0027,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Modified {
    pub delivery_failed: Option<Boolean>,
    pub undeliverable_here: Option<Boolean>,
    pub message_annotations: Option<Fields>,
}

#[cfg(test)]
mod tests {
    //! Test serialization and deserialization
    use fe2o3_amqp::{de::{Deserializer, from_slice}, format_code::EncodingCodes, read::SliceReader, ser::to_vec};
    use serde::Deserialize;

    use super::{Accepted, DeliveryState, Modified, Received, Rejected, Released};

    /* ---------------------------- // test Accepted ---------------------------- */
    #[test]
    fn test_serialize_deserialize_accepted() {
        let accepted = Accepted {};
        let buf = to_vec(&accepted).unwrap();
        let _: Accepted = from_slice(&buf).unwrap();
    }

    #[test]
    fn test_deserialize_accepted_from_list8() {
        // try deserialize from list8
        let buf = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallUlong as u8,
            0x24, // descriptor code
            EncodingCodes::List8 as u8,
            0, // size
            0, // count
        ];
        let _: Accepted = from_slice(&buf).unwrap();
    }

    #[test]
    fn test_deserialize_accepted_from_list32() {
        // try deserialize from list8
        let buf = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallUlong as u8,
            0x24, // descriptor code
            EncodingCodes::List32 as u8,
            0,
            0,
            0,
            0, // size
            0,
            0,
            0,
            0, // count
        ];
        let _: Accepted = from_slice(&buf).unwrap();
    }

    /* ------------------------------ test Rejected ----------------------------- */
    #[test]
    fn test_serialize_deserialize_rejected() {
        let rejected = Rejected { error: None };
        let buf = to_vec(&rejected).unwrap();
        let rejected: Rejected = from_slice(&buf).unwrap();
        assert!(rejected.error.is_none())
    }

    #[test]
    fn test_deserialize_rejected_from_list8() {
        // try deserialize from list8
        let buf = vec![
            EncodingCodes::DescribedType as u8,
            EncodingCodes::SmallUlong as u8,
            0x25, // descriptor code
            EncodingCodes::List8 as u8,
            0, // size
            0, // count
        ];
        let rejected: Rejected = from_slice(&buf).unwrap();
        assert!(rejected.error.is_none())
    }

    /* ------------------------------ test Released ----------------------------- */
    #[test]
    fn test_serialize_deserialize_released() {
        let released = Released {};
        let buf = to_vec(&released).unwrap();
        let _: Released = from_slice(&buf).unwrap();
    }

    /* ------------------------------ test Modified ----------------------------- */
    #[test]
    fn test_serialize_deserialize_modified() {
        let modified = Modified {
            delivery_failed: None,
            undeliverable_here: Some(true),
            message_annotations: None,
        };
        let buf = to_vec(&modified).unwrap();
        let modified2: Modified = from_slice(&buf).unwrap();
        println!("{:?}", buf);
        println!("{:?}", modified2);
    }

    /* ------------------------------ test Received ----------------------------- */
    #[test]
    fn test_serialize_deserialize_received() {
        let received = Received {
            section_number: 9,
            section_offset: 13
        };
        let buf = to_vec(&received).unwrap();
        let received2: Received = from_slice(&buf).unwrap();
        println!("{:?}", received2);
    }

    /* --------------------------- test DeliveryState --------------------------- */

    macro_rules! assert_delivery_state {
        ($value:ident, $expecting:path) => {
            match $value {
                $expecting(_) => {}
                _ => panic!("Wrong variant"),
            }
        };
    }

    #[test]
    fn test_serialize_deserialize_delivery_state() {
        // Accepted
        let state = DeliveryState::Accepted(Accepted {});
        let buf = to_vec(&state).unwrap();
        let state2: DeliveryState = from_slice(&buf).unwrap();
        assert_delivery_state!(state2, DeliveryState::Accepted);

        // Rejected
        let state = DeliveryState::Rejected(Rejected { error: None });
        let buf = to_vec(&state).unwrap();
        let state2: DeliveryState = from_slice(&buf).unwrap();
        assert_delivery_state!(state2, DeliveryState::Rejected);

        // Released
        let state = DeliveryState::Released(Released {});
        let buf = to_vec(&state).unwrap();
        let state2: DeliveryState = from_slice(&buf).unwrap();
        assert_delivery_state!(state2, DeliveryState::Released);

        // Modified
        let state = DeliveryState::Modified(Modified {
            delivery_failed: None,
            undeliverable_here: Some(true),
            message_annotations: None,
        });
        let buf = to_vec(&state).unwrap();
        println!("{:?}", buf);
        let state2: DeliveryState = from_slice(&buf).unwrap();
        assert_delivery_state!(state2, DeliveryState::Modified);
        if let DeliveryState::Modified(m) = state2 {
            assert_eq!(m.delivery_failed, None);
            assert_eq!(m.undeliverable_here, Some(true));
            assert!(m.message_annotations.is_none());
        }

        // Received
        let state = DeliveryState::Received(Received {
            section_number: 9,
            section_offset: 13,
        });
        let buf = to_vec(&state).unwrap();
        let state2 = from_slice(&buf).unwrap();
        assert_delivery_state!(state2, DeliveryState::Received);
        if let DeliveryState::Received(r) = state2 {
            assert_eq!(r.section_number, 9);
            assert_eq!(r.section_offset, 13);
        }
    }

    #[test]
    fn test_debug() {
        let state = DeliveryState::Modified(Modified {
            delivery_failed: None,
            undeliverable_here: Some(true),
            message_annotations: None,
        });
        let buf = to_vec(&state).unwrap();
        println!("{:?}", buf);
        let reader = SliceReader::new(&buf);
        let mut deserializer = Deserializer::new(reader);
        let modified = <DeliveryState as Deserialize>::deserialize(&mut deserializer).unwrap();
        println!("{:?}", modified);
    }
}
