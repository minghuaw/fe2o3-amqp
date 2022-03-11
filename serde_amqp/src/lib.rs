#![deny(missing_docs, missing_debug_implementations)]

//! A serde implementation of AMQP1.0 protocol.
//!
//! # Serializing and deserializing data structures
//!
//! Any type that implement `serde::Serialize` and `serde::Deserialize` trait can be serialized/deserialized with
//! the following convenience functions
//!
//! Serialization:
//!
//! - [`to_vec`]
//!
//! Deserialization:
//!
//! - [`from_slice`]
//! - [`from_reader`]
//!
//! # Primitive types
//!
//! All primitive types defined in AMQP1.0 protocol can be found in mod [primitives].
//!
//! # Described types
//!
//! AMQP1.0 specification allows annotating any AMQP type with a [`descriptor::Descriptor`], thus creating a described type.
//! Though this can be constructed using [`described::Described`] and [`Value`], a much easier way to define a custom described type is
//! to use the custom [`SerializeComposite`] and [`DeserializeComposite`] derive macros. Please be aware
//! that the `"derive"` feature flag must be enabled.
//!
//! You can read more about how to use the derive macros in the corresponding [section](#serializecomposite-and-deserializecomposite).
//!
//! # Untyped AMQP1.0 values
//!
//! Untyped AMQP1.0 values can be constructed as well as serialized/deserialized using [`Value`].
//!
//! ```rust
//! use serde_amqp::{
//!     SerializeComposite, DeserializeComposite, Value, to_vec, from_slice,
//!     described::Described, descriptor::Descriptor,
//! };
//!
//! #[derive(Debug, SerializeComposite, DeserializeComposite)]
//! #[amqp_contract(code = 0x13, encoding = "list")]
//! struct Foo(Option<bool>, Option<i32>);
//!
//! let foo = Foo(Some(true), Some(3));
//! let buf = to_vec(&foo).unwrap();
//! let value: Value = from_slice(&buf).unwrap();
//! let expected = Value::Described(
//!     Described {
//!         descriptor: Descriptor::Code(0x13),
//!         value: Box::new(Value::List(vec![
//!             Value::Bool(true),
//!             Value::Int(3)
//!         ]))
//!     }
//! );
//! assert_eq!(value, expected);
//! ```
//!
//! Types that implements `serde::Serialize` and `serde::Deserialize` traits can also be converted into/from
//! [`Value`] using the [`to_value`] and [`from_value`] functions.
//!
//! # **WARNING** `enum`
//!
//! `enum` in serde data model can be categorized as below.
//!
//! ```rust
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize)]
//! enum Enumeration {
//!     UnitVariant,
//!     NewTypeVariant(u32),
//!     TupleVariant(bool, u64, String),
//!     StructVariant { id: u32, is_true: bool },
//! }
//! ```
//!
//! AMQP1.0 protocol doesn't natively support `NewTypeVariant`, `TupleVariant` or `StructVariant`.
//! For the sake of completeness, serialization and deserialization for these variants are implemented as follows:
//!
//! - `NewTypeVariant` is encoded/decoded as a map of one key-value pair with the
//! variant index being the key and the single wrapped field being the value.
//! - `TupleVariant` is encoded/decoded as a map of one key-value pair with the
//! varant index being the key and a list of the fields being the value.
//! - `StructVariant` is encoded/decoded as a map of one key-value pair with the
//! variant index being the key and a list of the fields being the value.
//!
//! # Feature flag
//!
//! - `"derive"`: enables custom derive macros: `SerializeComposite` and `DeserializeComposite`.
//!
//! ## `SerializeComposite` and `DeserializeComposite`
//!
//! The macro provides three types of encodings:
//!
//! 1. `"list"`: The struct will be serialized as a described list. A described list is an AMQP1.0 list with its descriptor prepended to the list itself. The deserialization will take either the `"list"` or the `"map"` encoded values.
//! 2. `"map"`: The struct will be serialized as a described map. A described map is an AMQP1.0 map with its descriptor prepended to the map. The deserialization will take either the `"list"` or the `"map"` encoded values.
//! 3. `"basic"`: The struct must be a thin wrapper (containing only one field) over another serializable/deserializable type. The inner struct will be serialized/deserialized with the descriptor prepended to the struct.
//!
//! ### Details with the `"list"` encoding
//!
//! Optinal fields
//!
//! If a field is not marked with `"mandatory"` in the specification, the field can be an `Option`. During serialization, the optional fields may be skipped completely or encoded as an AMQP1.0 `null` primitive (`0x40`). During deserialization, an AMQP1.0 `null` primitive or an empty field will be decoded as a `None`.
//!
//! Fields with default values:
//!
//! For fields that have default values defined in the specification, the field type must implement both the `Default` and `PartialEq` trait. During serialization, if the field is equal to the default value of the field type, the field will be either ignored completely or encoded as an AMQP1.0 `null` primitive (`0x40`). During deserialization, an AMQP1.0 `null` primitive or an empty field will be decoded as the default value of the type.
//!
//! ## Example of the derive macros
//!
//! The `"list"` encoding will encode the `Attach` struct as a described list (a descriptor followed by a list of the fields).
//!
//! ```rust, ignore
//! /// 2.7.3 Attach
//! /// Attach a link to a session.
//! /// <type name="attach" class="composite" source="list" provides="frame">
//! ///     <descriptor name="amqp:attach:list" code="0x00000000:0x00000012"/>
//! #[derive(Debug, DeserializeComposite, SerializeComposite)]
//! #[amqp_contract(
//!     name = "amqp:attach:list",
//!     code = 0x0000_0000_0000_0012,
//!     encoding = "list",
//!     rename_all = "kebab-case"
//! )]
//! pub struct Attach {
//!     /// <field name="name" type="string" mandatory="true"/>
//!     pub name: String,
//!
//!     /// <field name="handle" type="handle" mandatory="true"/>
//!     pub handle: Handle,
//!
//!     /// <field name="role" type="role" mandatory="true"/>
//!     pub role: Role,
//!
//!     /// <field name="snd-settle-mode" type="sender-settle-mode" default="mixed"/>
//!     #[amqp_contract(default)]
//!     pub snd_settle_mode: SenderSettleMode,
//!
//!     /// <field name="rcv-settle-mode" type="receiver-settle-mode" default="first"/>
//!     #[amqp_contract(default)]
//!     pub rcv_settle_mode: ReceiverSettleMode,
//!
//!     /// <field name="source" type="*" requires="source"/>
//!     pub source: Option<Source>,
//!
//!     /// <field name="target" type="*" requires="target"/>
//!     pub target: Option<Target>,
//!
//!     /// <field name="unsettled" type="map"/>
//!     pub unsettled: Option<BTreeMap<DeliveryTag, DeliveryState>>,
//!
//!     /// <field name="incomplete-unsettled" type="boolean" default="false"/>
//!     #[amqp_contract(default)]
//!     pub incomplete_unsettled: Boolean,
//!
//!     /// <field name="initial-delivery-count" type="sequence-no"/>
//!     pub initial_delivery_count: Option<SequenceNo>,
//!
//!     /// <field name="max-message-size" type="ulong"/>
//!     pub max_message_size: Option<ULong>,
//!
//!     /// <field name="offered-capabilities" type="symbol" multiple="true"/>
//!     pub offered_capabilities: Option<Vec<Symbol>>,
//!
//!     /// <field name="desired-capabilities" type="symbol" multiple="true"/>
//!     pub desired_capabilities: Option<Vec<Symbol>>,
//!
//!     /// <field name="properties" type="fields"/>
//!     pub properties: Option<Fields>,
//! }
//! ```
//!
//! ```rust,ignore
//! /// 3.2.5 Application Properties
//! /// <type name="application-properties" class="restricted" source="map" provides="section">
//! ///     <descriptor name="amqp:application-properties:map" code="0x00000000:0x00000074"/>
//! /// </type>
//! #[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
//! #[amqp_contract(
//!     name = "amqp:application-properties:map",
//!     code = 0x0000_0000_0000_0074,
//!     encoding = "basic"
//! )]
//! pub struct ApplicationProperties(pub BTreeMap<String, SimpleValue>);
//! ```

// Public mods
pub mod de;
pub mod described;
pub mod descriptor;
pub mod error;
pub mod fixed_width;
pub mod format_code;
pub mod primitives;
pub mod read;
pub mod ser;
pub mod value;

// Private mod but is used by derive macros
// This is to avoid accidental misuse
#[doc(hidden)]
#[path = "constants.rs"]
pub mod __constants;

// Private mods
mod util;

// experimental
mod format;

pub use serde;

pub use de::{from_reader, from_slice};
pub use error::Error;
pub use ser::to_vec;
pub use value::{de::from_value, ser::to_value, Value};

#[cfg(feature = "derive")]
pub mod macros;
#[cfg(feature = "derive")]
pub use macros::{DeserializeComposite, SerializeComposite};
