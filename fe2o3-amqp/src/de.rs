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

pub struct CompoundBuf {
    count: u32,
    value_buf: Vec<u8>
}

pub struct ArrayBuf {
    count: u32,
    elem_code: u8, // element constructor
    value_buf: Vec<u8>,
}

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
                size as usize
            },
            VariableWidth::Four => {
                let size_buf = unpack!(self.reader.read_const_bytes::<4>());
                u32::from_be_bytes(size_buf) as usize
            }
        };

        self.reader.read_bytes(n)
    }

    fn read_compound_bytes(&mut self, width: CompoundWidth) -> Option<Result<CompoundBuf, Error>> {
        let cb = match width {
            CompoundWidth::Zero => {
                CompoundBuf { count: 0, value_buf: Vec::with_capacity(0) }
            },
            CompoundWidth::One => {
                let size = unpack!(self.reader.next());
                let count = unpack!(self.reader.next());

                // Need to subtract the one byte taken by `count`
                let size = size - 1;
                let value_buf = unpack!(self.reader.read_bytes(size as usize));
                CompoundBuf {count: count as u32, value_buf }
            },
            CompoundWidth::Four => {
                let size_buf = unpack!(self.reader.read_const_bytes::<4>());
                let count_buf = unpack!(self.reader.read_const_bytes::<4>());
                let size = u32::from_be_bytes(size_buf);
                let count = u32::from_be_bytes(count_buf);

                // Need to substract the four byte taken by `count`
                let size = size - 4;
                let value_buf = unpack!(self.reader.read_bytes(size as usize));
                CompoundBuf { count, value_buf }
            }
        };

        Some(Ok(cb))
    }

    fn read_array_bytes(&mut self, width: ArrayWidth) -> Option<Result<ArrayBuf, Error>> {
        let ab = match width {
            ArrayWidth::One => {
                let size = unpack!(self.reader.next());
                let count = unpack!(self.reader.next());
                let elem_code = unpack!(self.reader.next());

                // Must subtract the one byte taken by `count` and one byte taken by `elem_code`
                let size = size - 1 - 1;
                let value_buf = unpack!(self.reader.read_bytes(size as usize));
                ArrayBuf { count: count as u32, elem_code, value_buf }
            },
            ArrayWidth::Four => {
                let size_buf = unpack!(self.reader.read_const_bytes::<4>());
                let count_buf = unpack!(self.reader.read_const_bytes::<4>());
                let elem_code = unpack!(self.reader.next());

                let size = u32::from_be_bytes(size_buf);
                let count = u32::from_be_bytes(count_buf);

                // Must subtract the four bytes taken by `count` and one byte taken by `elem_code`
                let size = size - 4 - 1;
                let value_buf = unpack!(self.reader.read_bytes(size as usize));
                ArrayBuf { count, elem_code, value_buf }
            }
        };

        Some(Ok(ab))
    }
}
