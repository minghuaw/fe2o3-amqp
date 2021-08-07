use serde::{Serialize, Serializer};

use crate::contract::WithContract;

use crate::descriptor::Descriptor;

/// The described type will attach a descriptor before the value. 
/// There is no generic implementation of serialization. But a inner type
/// specific implementation will be generated via macro.
pub struct Described<'a, T: ?Sized> {
    pub descriptor: Descriptor,
    pub value: &'a T
}

impl<'a, T: WithContract + ?Sized> Described<'a, T> {
    pub fn new(value: &'a T) -> Self {
        value.with_contract()
    }
}

pub trait SerializeDescribed: WithContract + Serialize {
    type Ok;
    type Error;

    fn serialize_described<S: Serializer>(&self, se: S) -> Result<Self::Ok, Self::Error>;
}