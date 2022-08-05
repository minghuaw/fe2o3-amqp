//! Types defined in AMQP 1.0 specification Part 3: Messaging

use serde::{Deserialize, Serialize};
use serde_amqp::described::Described;
use serde_amqp::primitives::Array;
use serde_amqp::{primitives::Symbol, value::Value};
use std::collections::BTreeMap;

pub mod message;
pub use message::{Body, Message};

/* -------------------------- 3.2 Messaging Format -------------------------- */
mod format;
pub use format::*;

/* --------------------------- 3.4 Delivery State --------------------------- */
mod delivery_state;
pub use delivery_state::*;

/* -------------------------- 3.5 Source and Target ------------------------- */
mod source;
pub use source::{Source, SourceBuilder};

mod target;
pub use target::{Target, TargetArchetype, TargetBuilder};

mod terminus_durability;
pub use terminus_durability::TerminusDurability;

mod term_expiry_policy;
pub use term_expiry_policy::TerminusExpiryPolicy;

mod dist_mode;
pub use dist_mode::DistributionMode;

mod lifetime_policy;
pub use lifetime_policy::*;

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
/// maintained \[AMQPFILTERS\].
///
pub type FilterSet = BTreeMap<Symbol, Option<Described<Value>>>;

use crate::definitions::Fields;

/// 3.5.9 Node Properties
/// Properties of a node.
/// <type name="node-properties" class="restricted" source="fields"/>
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

/// The distribution modes that the node supports.
///
/// The value of this entry MUST be one or more symbols which are valid distribution-modes. That is,
/// the value MUST be of the same type as would be valid in a field defined with the following
/// attributes:
///
/// type=“symbol” multiple=“true” requires=“distribution-mode”
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SupportedDistModes(Array<DistributionMode>);

impl SupportedDistModes {
    /// Creates a "supported-dist-modes"
    ///
    /// The value of this entry MUST be one or more symbols which are valid distribution-modes.
    pub fn new(mode: DistributionMode) -> Self {
        Self(Array(vec![mode]))
    }

    /// Add a mode to the "supported-dist-modes"
    pub fn add_mode(&mut self, mode: DistributionMode) {
        self.0.push(mode)
    }

    /// Creates a "supported-dist-modes" from an iterator of `DistributionMode`. Returns an error if the iterator is empty
    pub fn try_from_modes(
        modes: impl IntoIterator<Item = DistributionMode>,
    ) -> Result<Self, Array<DistributionMode>> {
        let modes: Array<DistributionMode> = modes.into_iter().collect();
        if modes.0.is_empty() {
            Err(modes)
        } else {
            Ok(Self(modes))
        }
    }
}

impl From<DistributionMode> for SupportedDistModes {
    fn from(mode: DistributionMode) -> Self {
        Self(Array(vec![mode]))
    }
}

impl From<SupportedDistModes> for Fields {
    fn from(modes: SupportedDistModes) -> Self {
        let mut map = Fields::new();
        let values = modes
            .0
             .0
            .into_iter()
            .map(Symbol::from)
            .map(Value::Symbol)
            .collect();
        map.insert(Symbol::from("supported-dist-modes"), Value::Array(values));
        map
    }
}

impl From<SupportedDistModes> for Value {
    fn from(modes: SupportedDistModes) -> Self {
        let values: Array<Value> = modes
            .0
             .0
            .into_iter()
            .map(Symbol::from)
            .map(Value::Symbol)
            .collect();
        Value::Array(values)
    }
}
