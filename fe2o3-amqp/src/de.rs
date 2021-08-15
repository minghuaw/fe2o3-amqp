use crate::{error::Error, read::{IoReader, Read}};

pub fn from_slice<'de, T>(slice: &'de [u8]) -> Result<T, Error> {
    let io_reader = IoReader::new(slice);
    todo!()
}

pub struct Deserializer<'de, R> {
    reader: &'de R
}

impl<'de, R: Read<'de>> Deserializer<'de, R> {
    
}