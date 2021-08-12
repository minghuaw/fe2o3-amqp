use serde::{Serialize, Serializer};

use crate::descriptor::Descriptor;

pub const DESCRIBED_TYPE_MAGIC: &str = "DESCRIBED_TYPE_MAGIC";

/// The described type will attach a descriptor before the value. 
/// There is no generic implementation of serialization. But a inner type
/// specific implementation will be generated via macro.
pub struct Described<'a, T: ?Sized> {
    pub descriptor: Descriptor,
    pub value: &'a T
}