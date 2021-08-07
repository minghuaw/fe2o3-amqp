use super::Descriptor;

/// The described type will attach a descriptor before the value. 
/// There is no generic implementation of serialization. But a inner type
/// specific implementation will be generated via macro.
pub struct Described<'a, T> {
    pub descriptor: Descriptor,
    pub value: &'a T
}