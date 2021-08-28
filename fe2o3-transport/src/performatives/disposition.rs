use serde::{Deserialize, Serialize};
use fe2o3_amqp::{macros::Described, types::Boolean};

use crate::{DeliveryState, definitions::{DeliveryNumber, Role}};

#[derive(Debug, Deserialize, Serialize, Described)]
#[serde(rename_all = "kebab-case")]
#[amqp_contract(name="amqp:disposition:list", code=0x0000_0000_0000_0015, encoding="list")]
pub struct Disposition {
    role: Role,
    first: DeliveryNumber,
    last: Option<DeliveryNumber>,
    settled: Option<Boolean>,
    state: Option<DeliveryState>,
    batchable: Option<Boolean>
}