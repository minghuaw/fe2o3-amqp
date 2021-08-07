use crate::{described::Described, descriptor::Descriptor};

#[deprecated]
#[derive(Debug, Clone)]
pub enum EncodingType {
    List,
    Map
}

/// Implementing serialization with state will require specialization
/// and thus requires nightly
#[deprecated]
#[derive(Debug)]
pub struct Contract {
    pub descriptor: Descriptor,
    pub encoding_type: Option<EncodingType>,
}

#[deprecated]
impl Contract {
    pub fn get_descriptor(&self) -> &Descriptor {
        &self.descriptor
    }

    pub fn get_encoding_type(&self) -> &Option<EncodingType> {
        &self.encoding_type
    }
}

pub trait WithContract {
    fn with_contract<'a>(&'a self) -> Described<'a, Self>;
}