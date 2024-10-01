use crate::{descriptor::PeekDescriptor, value::de::ValueType};

#[derive(Debug, Default)]
pub(crate) enum SequenceType {
    #[default]
    List,
    Array,
    TransparentVec,
}

#[derive(Debug)]
pub(crate) enum NonNativeType {
    Dec32,
    Dec64,
    Dec128,
    Symbol,
    SymbolRef,
    Timestamp,
    Uuid,
}

#[derive(Debug, Clone)]
pub enum IsArrayElement {
    False,
    FirstElement,
    OtherElement,
}

#[derive(Debug, Clone)]
pub enum EnumType {
    None,
    Array,
    Descriptor,
    Value,
}

impl Default for EnumType {
    fn default() -> Self {
        Self::None
    }
}

/// Described type has the descriptor as the first field
#[derive(Debug)]
pub(crate) enum FieldRole {
    Descriptor,
    Fields,
}

#[derive(Debug, Clone)]
#[repr(u8)]
pub enum StructEncoding {
    None,
    DescribedList,
    DescribedMap,
    DescribedBasic,
}

impl Default for StructEncoding {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PeekTypeCode {
    Primitive(ValueType),
    Composite(PeekDescriptor),
}
