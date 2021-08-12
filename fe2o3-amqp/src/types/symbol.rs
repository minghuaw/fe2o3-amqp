use serde::Serialize;


pub const SYMBOL_MAGIC: &str = "SYMBOL";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Symbol(String);

impl Serialize for Symbol {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer 
    {
        serializer.serialize_newtype_struct(SYMBOL_MAGIC, self)    
    }
}