

// mod delivery_number;
// mod delivery_tag;
// mod error;
// mod fields;
// mod handle;
// mod ietf_lang_tag;

// mod milliseconds;
// mod msg_fmt;
// mod recver_settle_mode;
// mod role;
// mod seconds;
// mod sender_settle_mode;
// mod seq_no;

// mod transfer_number;

// pub use amqp_error::*;
// pub use conn_error::*;
// pub use constant_def::*;
// pub use delivery_number::*;
// pub use delivery_tag::*;
// pub use error::*;
// pub use fields::*;
// pub use handle::*;
// pub use ietf_lang_tag::*;
// pub use link_error::*;
// pub use milliseconds::*;
// pub use msg_fmt::*;
// pub use recver_settle_mode::*;
// pub use role::*;
// pub use seconds::*;
// pub use sender_settle_mode::*;
// pub use seq_no::*;
// pub use session_error::*;
// pub use transfer_number::*;

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;

use fe2o3_amqp::{types::{Symbol, Ubyte, Uint}, value::Value, macros::{Described, NonDescribed}};

/// 2.8.1 Role
#[derive(Debug, Deserialize, Serialize, NonDescribed)]
pub struct Role(bool);

/// 2.8.2 Sender Settle Mode
#[derive(Debug, Deserialize, Serialize, NonDescribed)]
pub struct SndSettleMode(Ubyte);

/// 2.8.3 Receiver Settle Mode
#[derive(Debug, Deserialize, Serialize, NonDescribed)]
pub struct RcvSettleMode(Ubyte);

/// 2.8.4 Handle
#[derive(Debug, Deserialize, Serialize, NonDescribed)]
pub struct Handle(Uint);

/// 2.8.5 Seconds
#[derive(Debug, Deserialize, Serialize, NonDescribed)]
pub struct Seconds(Uint);

/// 2.8.6 Milliseconds
#[derive(Debug, Serialize, Deserialize, NonDescribed)]
pub struct Milliseconds(Uint);

/// 2.8.7 Delivery Tag
#[derive(Debug, Deserialize, Serialize, NonDescribed, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeliveryTag(ByteBuf);

/// 2.8.8 Delivery Number
#[derive(Debug, Deserialize, Serialize, NonDescribed)]
pub struct DeliveryNumber(SequenceNo);

/// 2.8.9 Transfer Number
#[derive(Debug, Deserialize, Serialize, NonDescribed)]
pub struct TransferNumber(SequenceNo);

/// 2.8.10 Sequence No
#[derive(Debug, Deserialize, Serialize, NonDescribed)]
pub struct SequenceNo(Uint);

/// 2.8.11 Message Format
#[derive(Debug, Deserialize, Serialize, NonDescribed)]
pub struct MessageFormat(Uint);

/// 2.8.12 IETF Language Tag
#[derive(Debug, Serialize, Deserialize, NonDescribed)]
pub struct IetfLanguageTag(Symbol);

/// 2.8.13 Fields
#[derive(Debug, Serialize, Deserialize, NonDescribed)]
pub struct Fields(BTreeMap<Symbol, Value>);

/// 2.8.14 Error
#[derive(Debug, Deserialize, Serialize, Described)]
#[serde(rename_all = "kebab-case")]
#[amqp_contract(name="amqp:error:list", code=0x0000_0000_0000_001d, encoding="list")]
pub struct Error {
    condition: Symbol,
    description: Option<String>,
    info: Option<Fields>
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
pub use constant_def::{PORT, SECURE_PORT, MAJOR, MINOR, REVISION, MIN_MAX_FRAME_SIZE};