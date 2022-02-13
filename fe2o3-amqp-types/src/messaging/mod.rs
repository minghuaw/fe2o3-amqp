use serde_amqp::described::Described;
use serde_amqp::macros::{DeserializeComposite, SerializeComposite};
use serde_amqp::{primitives::Symbol, value::Value};
use std::collections::BTreeMap;

pub mod message;
pub use message::Message;

/* -------------------------- 3.2 Messaging Format -------------------------- */
mod format;
pub use format::*;

/* --------------------------- 3.4 Delivery State --------------------------- */
mod delivery_state;
pub use delivery_state::*;

/* -------------------------- 3.5 Source and Target ------------------------- */
pub mod source;
pub use source::Source;

pub mod target;
pub use target::Target;

pub mod terminus_durability;
pub use terminus_durability::TerminusDurability;

pub mod term_expiry_policy;
pub use term_expiry_policy::TerminusExpiryPolicy;

pub mod dist_mode;
pub use dist_mode::DistributionMode;

/// 3.5.8 Filter Set
/// <type name="filter-set" class="restricted" source="map"/>
///
/// A set of named filters. Every key in the map MUST be of type symbol,
/// every value MUST be either null or of a described type which provides
/// the archetype filter. A filter acts as a function on a message which
/// returns a boolean result indicating whether the message can pass through
/// that filter or not. A message will pass through a filter-set if and only
/// if it passes through each of the named filters. If the value for a given
/// key is null, this acts as if there were no such key present
/// (i.e., all messages pass through the null filter).
/// Filter types are a defined extension point. The filter types that a given
/// source supports will be indicated by the capabilities of the source.
/// A registry of commonly defined filter types and their capabilities is
/// maintained [AMQPFILTERS].
///
pub type FilterSet = BTreeMap<Symbol, Option<Described<Value>>>;

use crate::definitions::Fields;

/// 3.5.9 Node Properties
/// Properties of a node.
/// <type name="node-properties" class="restricted" source="fields"/>
// #[derive(Debug, Clone, Serialize, Deserialize)]
pub type NodeProperties = Fields;

// // The lifetime of a dynamically generated node.
// // Definitionally, the lifetime will never be less than the lifetime
// // of the link which caused its creation, however it is possible to
// // extend the lifetime of dynamically created node using a lifetime
// // policy. The value of this entry MUST be of a type which provides
// // the lifetime-policy archetype. The following standard lifetime-policies
// // are defined below: delete-on-close, delete-on-no-links,
// // delete-on-no-messages or delete-on-no-links-or-messages.
// //
// // TODO: impl Into Fields
// enum LifetimePolicy {
//     DeleteOnClose,
//     DeleteOnNoLinks,
//     DeleteOnNoMessages,
//     DeleteOnNoLinksOrMessages,
// }

// // TODO: impl into Fields
// struct SupportedDistMode(Vec<Symbol>);

/// 3.5.10 Delete On Close
/// Lifetime of dynamic node scoped to lifetime of link which caused creation.
/// <type name="delete-on-close" class="composite" source="list" provides="lifetime-policy">
///     <descriptor name="amqp:delete-on-close:list" code="0x00000000:0x0000002b"/>
/// </type>
#[derive(Debug, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "amqp:delete-on-close:list",
    code = 0x0000_0000_0000_002b,
    encoding = "list"
)]
pub struct DeleteOnClose {}

/// 3.5.11 Delete On No Links
/// Lifetime of dynamic node scoped to existence of links to the node
// <type name="delete-on-no-links" class="composite" source="list" provides="lifetime-policy">
//     <descriptor name="amqp:delete-on-no-links:list" code="0x00000000:0x0000002c"/>
// </type>
#[derive(Debug, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "amqp:delete-on-no-links:list",
    code = 0x0000_0000_0000_002c,
    encoding = "list"
)]
pub struct DeleteOnNoLinks {}

/// 3.5.12 Delete On No Messages
/// Lifetime of dynamic node scoped to existence of messages on the node.
/// <type name="delete-on-no-messages" class="composite" source="list" provides="lifetime-policy">
///     <descriptor name="amqp:delete-on-no-messages:list" code="0x00000000:0x0000002d"/>
/// </type>
#[derive(Debug, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "amqp:delete-on-no-messages:list",
    code = 0x0000_0000_0000_002d,
    encoding = "list"
)]
pub struct DeleteOnNoMessages {}

/// 3.5.13 Delete On No Links Or Messages
/// Lifetime of node scoped to existence of messages on or links to the node.
/// <type name="delete-on-no-links-or-messages" class="composite" source="list" provides="lifetime-policy">
///     <descriptor name="amqp:delete-on-no-links-or-messages:list" code="0x00000000:0x0000002e"/>
/// </type>
#[derive(Debug, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "amqp:delete-on-no-links-or-messages:list",
    code = 0x0000_0000_0000_002e,
    encoding = "list"
)]
pub struct DeleteOnNoLinksOrMessages {}
