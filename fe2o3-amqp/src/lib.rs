// Public mods
pub mod constants;
pub mod contract;
pub mod convert;
pub mod de;
pub mod error;
pub mod fixed_width;
pub mod format_code;
pub mod macros;
pub mod ser;
pub mod types;
pub mod value;

// Private mods
mod read;
mod util;

// experimental
mod format;

pub use serde;

pub use ser::to_vec;
pub use de::{from_reader, from_slice};
pub use value::{
    ser::to_value, de::from_value
};