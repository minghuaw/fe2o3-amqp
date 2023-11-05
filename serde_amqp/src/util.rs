use smallvec::{SmallVec, smallvec};

use crate::{descriptor::PeekDescriptor, value::de::ValueType};

#[derive(Debug)]
pub(crate) enum NewType {
    None,
    Array,
    Dec32,
    Dec64,
    Dec128,
    Symbol,
    SymbolRef,
    Timestamp,
    Uuid,
    TransparentVec,
}

impl Default for NewType {
    fn default() -> Self {
        Self::None
    }
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

pub trait TryFromSerializable<T: serde::ser::Serialize>: Sized {
    type Error: std::error::Error;

    fn try_from(value: T) -> Result<Self, Self::Error>;
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PeekTypeCode {
    Primitive(ValueType),
    Composite(PeekDescriptor),
}

#[derive(Debug, Clone)]
pub(crate) struct Stack<T> {
    inner: StackInner<T>,
}

#[derive(Debug, Clone)]
enum StackInner<T> {
    Empty,
    Single(T),
    Multiple(SmallVec<[T; 4]>),
}

impl<T> Stack<T> {
    pub fn new() -> Self {
        Self {
            inner: StackInner::Empty,
        }
    }

    pub fn push(&mut self, value: T) {
        match &mut self.inner {
            StackInner::Empty => self.inner = StackInner::Single(value),
            StackInner::Single(_) => {
                let old = std::mem::replace(&mut self.inner, StackInner::Empty);
                if let StackInner::Single(old) = old {
                    self.inner = StackInner::Multiple(smallvec![old, value]);
                } else {
                    unreachable!()
                }
            }
            StackInner::Multiple(vec) => vec.push(value),
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        match &mut self.inner {
            StackInner::Empty => None,
            StackInner::Single(_) => match std::mem::replace(&mut self.inner, StackInner::Empty) {
                StackInner::Single(value) => Some(value),
                _ => unreachable!(),
            }
            StackInner::Multiple(vec) => vec.pop(),
        }
    }

    pub fn peek(&self) -> Option<&T> {
        match &self.inner {
            StackInner::Empty => None,
            StackInner::Single(value) => Some(value),
            StackInner::Multiple(vec) => vec.last(),
        }
    }
}

#[derive(Debug)]
pub(crate) enum SequenceLength {
    Unknown,
    Known(usize),
}