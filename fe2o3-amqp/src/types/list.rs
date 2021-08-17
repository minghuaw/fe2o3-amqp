use serde::Serialize;


pub const LIST: &str = "LIST";

pub struct List<T>(pub Vec<T>);

impl<T> From<Vec<T>> for List<T> {
    fn from(val: Vec<T>) -> Self {
        Self(val)        
    }
}

impl<T: Serialize> Serialize for List<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer 
    {
        serializer.serialize_newtype_struct(LIST, &self.0)
    }
}