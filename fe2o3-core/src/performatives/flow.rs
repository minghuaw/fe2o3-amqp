use fe2o3_amqp::{
    macros::{DeserializeComposite, SerializeComposite},
    types::{Boolean, Uint},
};

use crate::definitions::{Fields, Handle, SequenceNo, TransferNumber};

#[derive(Debug, DeserializeComposite, SerializeComposite)]
// #[serde(rename_all = "kebab-case")]
#[amqp_contract(
    name = "amqp:flow:list",
    code = 0x0000_0000_0000_0013,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Flow {
    pub next_incoming_id: Option<TransferNumber>,
    pub incoming_window: Uint,
    pub next_outgoing_id: TransferNumber,
    pub outgoing_window: Uint,
    pub handle: Option<Handle>,
    pub delivery_count: Option<SequenceNo>,
    pub link_credit: Option<Uint>,
    pub available: Option<Uint>,
    #[amqp_contract(default)]
    pub drain: Boolean,
    #[amqp_contract(default)]
    pub echo: Boolean,
    pub properties: Option<Fields>,
}
