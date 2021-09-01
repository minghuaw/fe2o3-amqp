use fe2o3_amqp::types::Symbol;
use serde::{Serialize, Deserialize};
use fe2o3_amqp::macros::{SerializeComposite, DeserializeComposite};

/* -------------------------- 3.2 Messaging Format -------------------------- */
mod format;
pub use format::*;

/* --------------------------- 3.4 Delivery State --------------------------- */
mod delivery_state;
pub use delivery_state::*;

/* -------------------------- 3.5 Source and Target ------------------------- */
mod source;
pub use source::*;

mod target;
pub use target::*;

mod terminus_durability;
pub use terminus_durability::*;

mod term_expiry_policy;
pub use term_expiry_policy::*;

mod dist_mode;
pub use dist_mode::*;

mod filter_set;
pub use filter_set::*;

use crate::definitions::Fields;


/// 3.5.9 Node Properties
/// Properties of a node.
/// <type name="node-properties" class="restricted" source="fields"/>
#[derive(Debug, Serialize, Deserialize)]
pub struct NodeProperties(Fields);

// The lifetime of a dynamically generated node.
// Definitionally, the lifetime will never be less than the lifetime 
// of the link which caused its creation, however it is possible to 
// extend the lifetime of dynamically created node using a lifetime 
// policy. The value of this entry MUST be of a type which provides 
// the lifetime-policy archetype. The following standard lifetime-policies 
// are defined below: delete-on-close, delete-on-no-links, 
// delete-on-no-messages or delete-on-no-links-or-messages.
//
// TODO: impl Into Fields
pub enum LifetimePolicy {
    DeleteOnClose,
    DeleteOnNoLinks,
    DeleteOnNoMessages,
    DeleteOnNoLinksOrMessages,
}

// TODO: impl into Fields
pub struct SupportedDistMode(Vec<Symbol>);

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
pub struct DeleteOnClose { }

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
pub struct DeleteOnNoLinks { }

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
pub struct DeleteOnNoMessages { }

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
pub struct DeleteOnNoLinksOrMessages { }