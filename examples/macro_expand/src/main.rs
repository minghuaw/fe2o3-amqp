use serde_amqp::{macros::{SerializeComposite, DeserializeComposite}, primitives::Binary};

#[derive(SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "amqp:amqp-value:*",
    code = "0x0000_0000:0x0000_0077,
    encoding = "basic"
)]
pub struct AmqpValue<T>(pub T);

fn main() {

}
