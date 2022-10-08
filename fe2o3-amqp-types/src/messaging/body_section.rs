use serde::{ser, de};

use self::sealed::Sealed;

pub(crate) mod sealed {
    pub trait Sealed { }
}

/// Trait for message body
pub trait BodySection: Sealed + SerializableBodySection + DeserializableBodySection { }

impl<T> BodySection for T where T: Sealed + SerializableBodySection + DeserializableBodySection { }

/// Trait for a serializable body section
pub trait SerializableBodySection: Sealed {
    /// The serializable type
    type Serializable: ser::Serialize;

    /// Get a reference to the serializable type
    fn serializable(self) -> Self::Serializable;
}

/// Trait for a deserializable body section
pub trait DeserializableBodySection: Sealed {
    /// The deserializable type
    /// 
    /// TODO: change to GAT once it stablizes
    type Deserializable: de::DeserializeOwned;

    /// Convert from deserializable to self
    fn from_deserializable(deserializable: Self::Deserializable) -> Self;
}

pub trait IntoBodySection {
    type BodySection: BodySection;

    fn into_body_section(self) -> Self::BodySection;
}

pub trait IntoSerializableBodySection {
    type SerializableBodySection: SerializableBodySection;

    fn into_serializable_body_section(self) -> Self::SerializableBodySection;
}
