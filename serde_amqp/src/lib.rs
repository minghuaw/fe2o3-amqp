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
pub use value::{de::from_value, ser::to_value};

#[cfg(feature = "serde_amqp_derive")]
pub mod macros;
#[cfg(feature = "serde_amqp_derive")]
pub use macros::{DeserializeComposite, SerializeComposite};

pub mod prelude {
    pub use super::{from_reader, from_slice, from_value, to_value, to_vec, Error};

    #[cfg(feature = "serde_amqp_derive")]
    pub use super::macros::{DeserializeComposite, SerializeComposite};
}
