use fe2o3_amqp::{
    macros::{DeserializeComposite, SerializeComposite},
    types::Boolean,
};

use crate::{
    definitions::{DeliveryNumber, Role},
    messaging::DeliveryState,
};

#[derive(Debug, DeserializeComposite, SerializeComposite)]
// #[serde(rename_all = "kebab-case")]
#[amqp_contract(
    name = "amqp:disposition:list",
    code = 0x0000_0000_0000_0015,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Disposition {
    pub role: Role,
    pub first: DeliveryNumber,
    pub last: Option<DeliveryNumber>,
    pub settled: Option<Boolean>,
    pub state: Option<DeliveryState>,
    pub batchable: Option<Boolean>,
}
