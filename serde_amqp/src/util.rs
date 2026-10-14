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
    LazyValue,
}

#[derive(Debug, Clone)]
pub enum IsArrayElement {
    False,
    FirstElement,
    OtherElement,
}

#[derive(Debug, Clone, Default)]
pub enum EnumType {
    #[default]
    None,
    Array,
    Descriptor,
    Value,
}

/// Described type has the descriptor as the first field
#[derive(Debug)]
pub(crate) enum FieldRole {
    Descriptor,
    Fields,
}

#[derive(Debug, Clone, Default)]
#[repr(u8)]
pub enum StructEncoding {
    #[default]
    None,
    DescribedList,
    DescribedMap,
    DescribedBasic,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PeekTypeCode {
    Primitive(ValueType),
    Composite(PeekDescriptor),
}
