// Public mods
pub mod constants;
pub mod de;
pub mod described;
pub mod descriptor;
pub mod error;
pub mod fixed_width;
pub mod format_code;
pub mod macros;
pub mod primitives;
pub mod read;
pub mod ser;
pub mod value;

// Private mods
mod util;

// experimental
mod format;

pub use serde;

pub use de::{from_reader, from_slice};
pub use error::Error;
pub use macros::{DeserializeComposite, SerializeComposite};
pub use ser::to_vec;
pub use value::{de::from_value, ser::to_value};

pub mod prelude {
    pub use super::{
        from_reader, from_slice, from_value, to_value, to_vec, DeserializeComposite, Error,
        SerializeComposite,
    };
}
