use fe2o3_amqp::{
    macros::{DeserializeComposite, SerializeComposite},
    types::{Symbol, Uint, Ushort},
};

use crate::definitions::{Fields, Handle, TransferNumber};

#[derive(Debug, SerializeComposite, DeserializeComposite)]
// #[serde(rename_all = "kebab-case")]
#[amqp_contract(
    name = "amqp:begin:list",
    code = 0x0000_0000_0000_0011,
    encoding = "list",
    rename_field = "kebab-case"
)]
pub struct Begin {
    pub remote_channel: Option<Ushort>,
    pub next_outgoing_id: TransferNumber,
    pub incoming_window: Uint,
    pub outgoing_window: Uint,
    pub handle_max: Option<Handle>,
    pub offered_capabilities: Option<Vec<Symbol>>,
    pub desired_capabilities: Option<Vec<Symbol>>,
    pub properties: Option<Fields>,
}
