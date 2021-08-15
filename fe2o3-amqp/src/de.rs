use std::convert::TryInto;

use crate::{constructor::EncodingCodes, error::Error, read::{IoReader, Read}, unpack};

pub fn from_slice<'de, T>(slice: &'de [u8]) -> Result<T, Error> {
    let io_reader = IoReader::new(slice);
    todo!()
}

// pub struct ItemBytes {
//     constructor: EncodingCodes,
//     size: Option<Vec<u8>>,
//     count: Option<Vec<u8>>,
//     content: Option<Vec<u8>>,
// }

// pub enum ItemBytes {
//     Described {
//         descriptor_bytes: Vec<u8>,
//         value_bytes: Vec<u8>
//     },
//     Fixed {

//     },
// }

pub struct Deserializer<R> {
    reader: R
}

impl<'de, R: Read<'de>> Deserializer<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    fn read_constructor(&mut self) -> Option<Result<EncodingCodes, Error>> {
        let code = self.reader.next();
        let code = unpack!(code);
        Some(code.try_into())
    }
}