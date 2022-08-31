use std::{borrow::Borrow, collections::BTreeMap};

use serde_amqp::{primitives::{ULong, Symbol, SymbolRef}, Value};

pub type Annotations = BTreeMap<OwnedKey, Value>;

#[derive(Debug)]
pub enum OwnedKey {
    Symbol(Symbol),
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

impl<'a> Borrow<dyn AnnotationKey + 'a> for OwnedKey {
    fn borrow(&self) -> &(dyn AnnotationKey + 'a) {
        self
    }
}

