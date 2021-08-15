use std::convert::TryInto;

use crate::{
    constructor::EncodingCodes,
    error::Error,
    format::{ArrayWidth, CompoundWidth, FixedWidth, VariableWidth},
    read::{IoReader, Read},
    unpack,
};

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

// pub enum Content {
//     Described {
//         descriptor_buf: Vec<u8>,
//         value_buf: Vec<u8>
//     },
//     Fixed {
//         buf: Vec<u8>
//     },
//     Variable {
//         buf: Vec<u8>
//     },
//     Compound {
//         buf
//     }
// }

pub struct Deserializer<R> {
    reader: R,
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

    fn read_fixed_width_bytes(&mut self, width: FixedWidth) -> Option<Result<Vec<u8>, Error>> {
        let n = width as usize;
        self.reader.read_bytes(n)
    }

    fn read_variable_width_bytes(&mut self, width: VariableWidth) -> Option<Result<Vec<u8>, Error>> {
        let n = match width {
            VariableWidth::One => {
                let size = unpack!(self.reader.next());
                u8::from_be_bytes([size]) as usize
            },
            VariableWidth::Four => {
                let size_buf = unpack!(self.reader.read_bytes(4));
                u32::from_be_bytes(size_buf) as usize
            }
        };
    }

    fn read_compound_bytes(&mut self, width: CompoundWidth) -> Option<Result<Vec<u8>, Error>> {
        todo!()
    }

    fn read_array_bytes(&mut self, width: ArrayWidth) -> Option<Result<Vec<u8>, Error>> {
        todo!()
    }
}
