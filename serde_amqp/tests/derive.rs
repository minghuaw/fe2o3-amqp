//! This only acts as a test and macro expansion for the derive macro
//!
//! cargo expand --test derive --features "derive"
#[cfg(feature = "derive")]
use serde_amqp::{DeserializeComposite, SerializeComposite};

#[cfg(feature = "derive")]
#[derive(SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "test:example:bool",
    code = "0x0000_0000:0x0000_0075",
    encoding = "list"
)]
struct Example {
    a: i32,
}
