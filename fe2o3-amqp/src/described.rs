use serde::ser::{Serialize, Serializer, SerializeStruct};

use crate::descriptor::Descriptor;

pub const DESCRIBED_TYPE: &str = "DESCRIBED";

/// The described type will attach a descriptor before the value. 
/// There is no generic implementation of serialization. But a inner type
/// specific implementation will be generated via macro.
pub struct Described<'a, T: ?Sized> {
    pub descriptor: Descriptor,
    pub value: &'a T
}

impl<'a, T: ?Sized + Serialize> Serialize for Described<'a, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer 
    {
        let mut state = serializer.serialize_struct(DESCRIBED_TYPE, 2)?;
        state.serialize_field("descriptor", &self.descriptor)?;
        state.serialize_field("value", &self.value)?;
        state.end()
    }
}

// #[cfg(test)]
// mod tests {
//     #[test]
//     fn 
// }