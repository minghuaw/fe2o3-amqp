use fe2o3_amqp::{macros::{DeserializeDescribed, SerializeDescribed}, types::Boolean};

use crate::{DeliveryState, definitions::{DeliveryNumber, Role}};

#[derive(Debug, DeserializeDescribed, SerializeDescribed)]
// #[serde(rename_all = "kebab-case")]
#[amqp_contract(name="amqp:disposition:list", code=0x0000_0000_0000_0015, encoding="list")]
pub struct Disposition {
    role: Role,
    first: DeliveryNumber,
    last: Option<DeliveryNumber>,
    settled: Option<Boolean>,
    state: Option<DeliveryState>,
    batchable: Option<Boolean>
}