use serde::{Serialize, Deserialize};
use fe2o3_amqp::types::{Boolean, Uint, Ulong};
use fe2o3_amqp::macros::{DeserializeComposite, SerializeComposite};

use crate::definitions::{Error, Fields};


/// 3.4.1 Received
///
/// <type name="received" class="composite" source="list" provides="delivery-state">
/// <descriptor name="amqp:received:list" code="0x00000000:0x00000023"/>
/// <field name="section-number" type="uint" mandatory="true"/>
/// <field name="section-offset" type="ulong" mandatory="true"/>
/// </type>
#[derive(Debug, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name="amqp:received:list",
    code=0x0000_0000_0000_0023,
    encoding="list",
    rename_field="kebab-case"
)]
pub struct Received {
    section_number: Uint,
    section_offset: Ulong
}

/// 3.4.2 Accepted
/// The accepted outcome
///
/// <type name="accepted" class="composite" source="list" provides="delivery-state, outcome">
///     <descriptor name="amqp:accepted:list" code="0x00000000:0x00000024"/>
/// </type>
#[derive(Debug, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:accepted:list",
    code = 0x0000_0000_0000_0024,
    encoding = "list",
    rename_field = "kebab-case"
)]
pub struct Accepted { }

/// 3.4.3 Rejected
/// The rejected outcome.
///
/// <type name="rejected" class="composite" source="list" provides="delivery-state, outcome">
///     <descriptor name="amqp:rejected:list" code="0x00000000:0x00000025"/>
///     <field name="error" type="error"/>
/// </type>
#[derive(Debug, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:rejected:list",
    code = 0x0000_0000_0000_0025,
    encoding = "list",
    rename_field = "kebab-case"
)]
pub struct Rejected {
    error: Option<Error>
}

/// 3.4.4 Released
/// The released outcome.
/// <type name="released" class="composite" source="list" provides="delivery-state, outcome">
///     <descriptor name="amqp:released:list" code="0x00000000:0x00000026"/>
/// </type>
#[derive(Debug, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:released:list",
    code = 0x000_0000_0000_0026,
    encoding = "list",
    rename_field = "kebab-case"
)]
pub struct Released { }

/// 3.4.5 Modified
/// The modified outcome.
/// <type name="modified" class="composite" source="list" provides="delivery-state, outcome">
///     <descriptor name="amqp:modified:list" code="0x00000000:0x00000027"/>
///     <field name="delivery-failed" type="boolean"/>
///     <field name="undeliverable-here" type="boolean"/>
///     <field name="message-annotations" type="fields"/>
/// </type>
#[derive(Debug, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:modified:list",
    code = 0x0000_0000_0000_0027,
    encoding = "list",
    rename_field = "kebab-case"
)]
pub struct Modified {
    delivery_failed: Option<Boolean>,
    undeliverable_here: Option<Boolean>,
    message_annotations: Option<Fields>
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum DeliveryState {
    Accepted(Accepted),
    Rejected(Rejected),
    Released(Released),
    Modified(Modified),
    Received(Received),
}

#[cfg(test)]
mod tests {
    //! Test serialization and deserialization
    use fe2o3_amqp::{ser::to_vec, de::from_slice, format_code::EncodingCodes};

    use super::Accepted;

    /* ---------------------------- // test Accepted ---------------------------- */
    #[test]
    fn test_serialize_deserialize_accepted() {
        let accepted = Accepted { };
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
            0, 0, 0, 0, // size
            0, 0, 0, 0, // count
        ];
        let _: Accepted = from_slice(&buf).unwrap();
    }

    /* ------------------------------ test Rejected ----------------------------- */
    #[test]
    fn test_serialize_deserialize_rejected() {
        
    }
}