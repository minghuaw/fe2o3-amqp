use fe2o3_amqp::{
    macros::{DeserializeComposite, SerializeComposite},
    types::Boolean,
};

use crate::{
    definitions::{DeliveryNumber, Role},
    DeliveryState,
};

#[derive(Debug, DeserializeComposite, SerializeComposite)]
// #[serde(rename_all = "kebab-case")]
#[amqp_contract(
    name = "amqp:disposition:list",
    code = 0x0000_0000_0000_0015,
    encoding = "list",
    rename_field = "kebab-case"
)]
pub struct Disposition {
    role: Role,
    first: DeliveryNumber,
    last: Option<DeliveryNumber>,
    settled: Option<Boolean>,
    state: Option<DeliveryState>,
    batchable: Option<Boolean>,
}
