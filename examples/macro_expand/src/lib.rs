// use std::collections::BTreeMap;

// use fe2o3_amqp::macros::{DeserializeComposite, SerializeComposite};

// #[derive(SerializeComposite, DeserializeComposite)]
// #[amqp_contract(code = 0x13, encoding = "map")]
// struct Foo {
//     is_fool: Option<bool>,
//     #[amqp_contract(default)]
//     a: i32,
// }

// #[derive(SerializeComposite, DeserializeComposite)]
// #[amqp_contract(encoding="list")]
// struct Unit { }

// #[derive(SerializeComposite, DeserializeComposite)]
// struct TupleStruct(Option<i32>, bool);

// #[derive(Debug, SerializeComposite, DeserializeComposite)]
// #[amqp_contract(code = 0x01, encoding = "basic")]
// struct Wrapper {
//     map: BTreeMap<String, i32>,
// }

// #[derive(Debug, SerializeComposite, DeserializeComposite)]
// #[amqp_contract(name = "ab", encoding = "list")]
// struct Test {
//     a: i32,
//     b: bool,
// }

use std::collections::BTreeMap;

use fe2o3_amqp::macros::{DeserializeComposite, SerializeComposite};
use fe2o3_amqp::primitives::{Boolean, Uint, Ulong};
use serde::{Deserialize, Serialize};

// use crate::definitions::{Error, Fields};

/// 3.4 Delivery State
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeliveryState {
    Accepted(Accepted),
    Rejected(Rejected),
    Released(Released),
    Modified(Modified),
    Received(Received),
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
    pub error: Option<bool>,
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
    pub message_annotations: Option<BTreeMap<String, String>>,
}
