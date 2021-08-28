use serde::{Deserialize, Serialize};
use fe2o3_amqp::{macros::Described, types::{Boolean, Uint}};

use crate::definitions::{Fields, Handle, SequenceNo, TransferNumber};

#[derive(Debug, Deserialize, Serialize, Described)]
#[serde(rename_all = "kebab-case")]
#[amqp_contract(name="amqp:flow:list", code=0x0000_0000_0000_0013, encoding="list")]
pub struct Flow {
    next_incoming_id: Option<TransferNumber>,
    incoming_window: Uint,
    next_outgoing_id: TransferNumber,
    outgoing_window: Uint,
    handle: Option<Handle>,
    delivery_count: Option<SequenceNo>,
    link_credit: Option<Uint>,
    available: Option<Uint>,
    drain: Option<Boolean>,
    echo: Option<Boolean>,
    properties: Option<Fields>
}