// use std::collections::BTreeMap;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_amqp::{
    macros::{DeserializeComposite, SerializeComposite},
    primitives::{Boolean, UInt, ULong},
};

// #[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
// #[amqp_contract(
//     name = "amqp:amqp-value:*",
//     code = 0x0000_0000_0000_0077,
//     encoding = "list"
// )]
// // pub struct AmqpValue<T>(pub T);
// pub struct Foo<A, B> {
//     a: A,
//     b: B,
// }
// pub struct A(i32);

#[derive(Serialize, Deserialize)]
pub struct AnotherNewType<T> {
    inner: T,
}

// #[derive(SerializeComposite, DeserializeComposite)]
// #[amqp_contract(code = 0x13, encoding = "list", rename_all = "kebab-case")]
// struct Foo {
//     b: u64,
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
// #[amqp_contract(code = 0x01, encoding = "basic")]
// struct Wrapper(BTreeMap<String, i32>);

// #[derive(Debug, SerializeComposite, DeserializeComposite)]
// #[amqp_contract(name = "ab", encoding = "map")]
// struct Test {
//     a: Option<i32>,
//     #[amqp_contract(default)]
//     b: bool,
// }

// use std::collections::BTreeMap;

// use serde::{Deserialize, Serialize};
// use serde_amqp::macros::{DeserializeComposite, SerializeComposite};
// use serde_amqp::primitives::{Boolean, UInt, ULong};

// // use crate::definitions::{Error, Fields};

// /// 3.4 Delivery State
// #[derive(Debug, Clone, Serialize, Deserialize)]
// #[serde(untagged)]
// pub enum DeliveryState {
//     Accepted(Accepted),
//     Rejected(Rejected),
//     Released(Released),
//     Modified(Modified),
//     Received(Received),
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// #[serde(untagged)]
// pub enum Outcome {
//     Accepted(Accepted),
//     Rejected(Rejected),
//     Released(Released),
//     Modified(Modified),
// }

// /// 3.4.1 Received
// ///
// /// <type name="received" class="composite" source="list" provides="delivery-state">
// /// <descriptor name="amqp:received:list" code="0x00000000:0x00000023"/>
// /// </type>
// #[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
// #[amqp_contract(
//     name = "amqp:received:list",
//     code = 0x0000_0000_0000_0023,
//     encoding = "list",
//     rename_all = "kebab-case"
// )]
// pub struct Received {
//     /// <field name="section-number" type="uint" mandatory="true"/>
//     pub section_number: UInt,

//     /// <field name="section-offset" type="ulong" mandatory="true"/>
//     pub section_offset: ULong,
// }

// /// 3.4.2 Accepted
// /// The accepted outcome
// ///
// /// <type name="accepted" class="composite" source="list" provides="delivery-state, outcome">
// ///     <descriptor name="amqp:accepted:list" code="0x00000000:0x00000024"/>
// /// </type>
// #[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
// #[amqp_contract(
//     name = "amqp:accepted:list",
//     code = 0x0000_0000_0000_0024,
//     encoding = "list",
//     rename_all = "kebab-case"
// )]
// pub struct Accepted {}

// /// 3.4.3 Rejected
// /// The rejected outcome.
// ///
// /// <type name="rejected" class="composite" source="list" provides="delivery-state, outcome">
// ///     <descriptor name="amqp:rejected:list" code="0x00000000:0x00000025"/>
// /// </type>
// #[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
// #[amqp_contract(
//     name = "amqp:rejected:list",
//     code = 0x0000_0000_0000_0025,
//     encoding = "list",
//     rename_all = "kebab-case"
// )]
// pub struct Rejected {
//     /// <field name="error" type="error"/>
//     pub error: Option<bool>,
// }

// /// 3.4.4 Released
// /// The released outcome.
// /// <type name="released" class="composite" source="list" provides="delivery-state, outcome">
// ///     <descriptor name="amqp:released:list" code="0x00000000:0x00000026"/>
// /// </type>
// #[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
// #[amqp_contract(
//     name = "amqp:released:list",
//     code = 0x000_0000_0000_0026,
//     encoding = "list",
//     rename_all = "kebab-case"
// )]
// pub struct Released {}

// /// 3.4.5 Modified
// /// The modified outcome.
// /// <type name="modified" class="composite" source="list" provides="delivery-state, outcome">
// ///     <descriptor name="amqp:modified:list" code="0x00000000:0x00000027"/>
// /// </type>
// #[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
// #[amqp_contract(
//     name = "amqp:modified:list",
//     code = 0x0000_0000_0000_0027,
//     encoding = "list",
//     rename_all = "kebab-case"
// )]
// pub struct Modified {
//     /// <field name="delivery-failed" type="boolean"/>
//     pub delivery_failed: Option<Boolean>,

//     /// <field name="undeliverable-here" type="boolean"/>
//     pub undeliverable_here: Option<Boolean>,

//     /// <field name="message-annotations" type="fields"/>
//     pub message_annotations: Option<BTreeMap<String, String>>,
// }
