// mod amqp_error;
// mod conn_error;
// mod constant_def;
// mod delivery_number;
// mod delivery_tag;
// mod error;
// mod fields;
// mod handle;
// mod ietf_lang_tag;
// mod link_error;
// mod milliseconds;
// mod msg_fmt;
// mod recver_settle_mode;
// mod role;
// mod seconds;
// mod sender_settle_mode;
// mod seq_no;
// mod session_error;
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

use fe2o3_amqp::{types::{Symbol, Ubyte, Uint}, value::Value};

/// 2.8.1 Role
#[derive(Debug, Deserialize, Serialize)]
pub struct Role(bool);

/// 2.8.2 Sender Settle Mode
#[derive(Debug, Deserialize, Serialize)]
pub struct SndSettleMode(Ubyte);

/// 2.8.3 Receiver Settle Mode
#[derive(Debug, Deserialize, Serialize)]
pub struct RcvSettleMode(Ubyte);

/// 2.8.4 Handle
#[derive(Debug, Deserialize, Serialize)]
pub struct Handle(Uint);

/// 2.8.6 Milliseconds
#[derive(Debug, Serialize, Deserialize)]
pub struct Milliseconds(Uint);

/// 2.8.7 Delivery Tag
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeliveryTag(ByteBuf);

/// 2.8.9 Transfer Number
#[derive(Debug, Deserialize, Serialize)]
pub struct TransferNumber(SequenceNo);

/// 2.8.10 Sequence No
#[derive(Debug, Deserialize, Serialize)]
pub struct SequenceNo(Uint);

/// 2.8.12 IETF Language Tag
#[derive(Debug, Serialize, Deserialize)]
pub struct IetfLanguageTag(Symbol);

/// 2.8.13 Fields
#[derive(Debug, Serialize, Deserialize)]
pub struct Fields(BTreeMap<Symbol, Value>);