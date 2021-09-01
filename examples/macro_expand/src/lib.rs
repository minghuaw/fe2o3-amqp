use std::collections::BTreeMap;

use fe2o3_amqp::macros::{DeserializeComposite, SerializeComposite};

// #[derive(SerializeComposite, DeserializeComposite)]
// #[amqp_contract(code = 0x13, encoding = "list")]
// struct Foo {
//     is_fool: Option<bool>,
//     a: Option<i32>,
// }

// #[derive(SerializeComposite, DeserializeComposite)]
// #[amqp_contract(encoding="list")]
// struct Unit { }

// #[derive(SerializeComposite, DeserializeComposite)]
// struct TupleStruct(Option<i32>, bool);

#[derive(Debug, SerializeComposite, DeserializeComposite)]
#[amqp_contract(code = 0x01, encoding = "basic")]
struct Wrapper {
    map: BTreeMap<String, i32>,
}
