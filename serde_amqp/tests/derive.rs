use serde_amqp::{SerializeComposite, DeserializeComposite};

#[derive(SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "test:example:bool",
    code = "0x0000_0000:0x0000_0075",
    encoding = "list"
)]
struct Example {
    a: i32
}