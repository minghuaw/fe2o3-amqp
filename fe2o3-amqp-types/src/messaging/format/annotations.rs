use std::{borrow::Borrow, collections::BTreeMap, hash::{Hash, Hasher}};

use serde_amqp::{primitives::{ULong, Symbol, SymbolRef}, Value};

pub type Annotations = BTreeMap<OwnedKey, Value>;

/// Key type for [`Annotations`]
#[derive(Debug)]
pub enum OwnedKey {
    /// Symbol
    Symbol(Symbol),

    /// ULong
    ULong(ULong),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BorrowedKey<'k> {
    Symbol(SymbolRef<'k>),
    ULong(&'k u64)
}

pub trait AnnotationKey {
    fn key<'k>(&'k self) -> BorrowedKey<'k>;
}

impl<'a> AnnotationKey for BorrowedKey<'a> {
    fn key<'k>(&'k self) -> BorrowedKey<'k> {
        self.clone()
    }
}

impl AnnotationKey for OwnedKey {
    fn key<'k>(&'k self) -> BorrowedKey<'k> {
        match self {
            OwnedKey::Symbol(Symbol(s)) => BorrowedKey::Symbol(SymbolRef(s)),
            OwnedKey::ULong(v) => BorrowedKey::ULong(v),
        }
    }
}

impl AnnotationKey for u64 {
    fn key<'k>(&'k self) -> BorrowedKey<'k> {
        BorrowedKey::ULong(self)
    }
}

impl AnnotationKey for str {
    fn key<'k>(&'k self) -> BorrowedKey<'k> {
        BorrowedKey::Symbol(SymbolRef(self))
    }
}

impl AnnotationKey for String {
    fn key<'k>(&'k self) -> BorrowedKey<'k> {
        BorrowedKey::Symbol(SymbolRef(self))
    }
}

impl AnnotationKey for Symbol {
    fn key<'k>(&'k self) -> BorrowedKey<'k> {
        BorrowedKey::Symbol(SymbolRef(&self.0))
    }
}

impl<'a> AnnotationKey for SymbolRef<'a> {
    fn key<'k>(&'k self) -> BorrowedKey<'k> {
        BorrowedKey::Symbol(SymbolRef(self.0))
    }
}

impl<'a> Borrow<dyn AnnotationKey + 'a> for OwnedKey {
    fn borrow(&self) -> &(dyn AnnotationKey + 'a) {
        self
    }
}

impl<'a> PartialEq for (dyn AnnotationKey + 'a) {
    fn eq(&self, other: &Self) -> bool {
        self.key().eq(&other.key())
    }
}

impl<'a> Eq for (dyn AnnotationKey + 'a) { }

impl<'a> PartialOrd for (dyn AnnotationKey + 'a) {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.key().partial_cmp(&other.key())
    }
}

impl<'a> Ord for (dyn AnnotationKey + 'a) {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key().cmp(&other.key())
    }
}

impl<'a> Hash for (dyn AnnotationKey + 'a) {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key().hash(state)
    }
}