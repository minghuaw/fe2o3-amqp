//! This only acts as a test and macro expansion for the derive macro
//!
//! cargo expand --test derive --features "derive"
#[cfg(feature = "derive")]
use serde_amqp::{DeserializeComposite, SerializeComposite};

// #[cfg(feature = "derive")]
// #[derive(SerializeComposite, DeserializeComposite)]
// #[amqp_contract(
//     name = "test:example:bool",
//     code = "0x0000_0000:0x0000_0075",
//     encoding = "list"
// )]
// struct Example {
//     a: i32,
// }

#[derive(DeserializeComposite)]
#[amqp_contract(name = "a", encoding = "list", rename_all = "kebab-case")]
struct ExampleMap {
    a: i32,
    b: u32,
    c: String,
    #[amqp_contract(default)]
    foo_bar: u8,
}

// #[test]
// fn test_serialize_example_map() {
//     let example = ExampleMap {
//         a: 1,
//         b: 2,
//         c: "hello".to_string(),
//         foo_bar: 0,
//     };
//     let encoded = serde_amqp::to_vec(&example).unwrap();
//     println!("encoded: {:#x?}", encoded);
//     let decoded: ExampleMap = serde_amqp::from_slice(&encoded).unwrap();
// }