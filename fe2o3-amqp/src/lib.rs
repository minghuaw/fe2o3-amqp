// Public mods
pub mod constants;
pub mod de;
pub mod error;
pub mod fixed_width;
pub mod format_code;
pub mod macros;
pub mod ser;
pub mod primitives;
pub mod value;
pub mod described;
pub mod descriptor;
pub mod read;

// Private mods
mod util;

// experimental
mod format;

pub use serde;

pub use de::{from_reader, from_slice};
pub use ser::to_vec;
pub use value::{de::from_value, ser::to_value};
