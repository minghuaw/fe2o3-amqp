use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use std::collections::BTreeMap;

use fe2o3_amqp::{
    macros::{DeserializeComposite, SerializeComposite},
    types::{Symbol, Ubyte, Uint},
    value::Value,
};

/// 2.8.1 Role
mod role;
pub use role::Role;

/// 2.8.2 Sender Settle Mode
#[derive(Debug, Deserialize, Serialize)]
pub struct SenderSettleMode(Ubyte);

/// 2.8.3 Receiver Settle Mode
#[derive(Debug, Deserialize, Serialize)]
pub struct ReceivervSettleMode(Ubyte);

/// 2.8.4 Handle
#[derive(Debug, Deserialize, Serialize)]
pub struct Handle(Uint);

/// 2.8.5 Seconds
#[derive(Debug, Deserialize, Serialize)]
pub struct Seconds(Uint);

/// 2.8.6 Milliseconds
#[derive(Debug, Serialize, Deserialize)]
pub struct Milliseconds(Uint);

/// 2.8.7 Delivery Tag
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeliveryTag(ByteBuf);

/// 2.8.8 Delivery Number
#[derive(Debug, Deserialize, Serialize)]
pub struct DeliveryNumber(SequenceNo);

/// 2.8.9 Transfer Number
#[derive(Debug, Deserialize, Serialize)]
pub struct TransferNumber(SequenceNo);

/// 2.8.10 Sequence No
#[derive(Debug, Deserialize, Serialize)]
pub struct SequenceNo(Uint);

/// 2.8.11 Message Format
#[derive(Debug, Deserialize, Serialize)]
pub struct MessageFormat(Uint);

/// 2.8.12 IETF Language Tag
#[derive(Debug, Serialize, Deserialize)]
pub struct IetfLanguageTag(Symbol);

/// 2.8.13 Fields
#[derive(Debug, Serialize, Deserialize)]
pub struct Fields(BTreeMap<Symbol, Value>);

/// 2.8.14 Error
#[derive(Debug, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "amqp:error:list",
    code = 0x0000_0000_0000_001d,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Error {
    condition: Symbol,
    description: Option<String>,
    info: Option<Fields>,
}

/// 2.8.15 AMQP Error
mod amqp_error;
pub use amqp_error::AmqpError;

/// 2.8.16 Connection Error
mod conn_error;
pub use conn_error::ConnectionError;

/// 2.8.17 Session Error
mod session_error;
pub use session_error::SessionError;

/// 2.8.18 Link Error
mod link_error;
pub use link_error::LinkError;

/// 2.8.19 Constant definition
mod constant_def;
pub use constant_def::{MAJOR, MINOR, MIN_MAX_FRAME_SIZE, PORT, REVISION, SECURE_PORT};


