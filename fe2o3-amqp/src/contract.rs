use crate::types::Descriptor;

#[derive(Debug, Clone)]
pub enum EncodingType {
    List,
    Map
}

#[derive(Debug)]
pub struct Contract {
    pub descriptor: Descriptor,
    pub encoding_type: Option<EncodingType>,
}

impl Contract {
    pub fn get_descriptor(&self) -> &Descriptor {
        &self.descriptor
    }

    pub fn get_encoding_type(&self) -> &Option<EncodingType> {
        &self.encoding_type
    }
}
