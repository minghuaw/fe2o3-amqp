use std::io;

pub struct Deserializer<'de, R> {
    reader: &'de R
}

impl<'de, R: io::Read> Deserializer<'de, R> {
    
}