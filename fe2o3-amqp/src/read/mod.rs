use std::convert::TryInto;

use crate::{error::Error, format::{EncodedWidth}, format_code::EncodingCodes};

mod ioread;
pub use ioread::*;

mod sliceread;
pub use sliceread::*;

mod private {
    pub trait Sealed {}
}

pub trait Read<'de>: private::Sealed {
    /// Peek the next byte without consuming
    fn peek(&mut self) -> Result<u8, Error>;

    /// Read the next byte
    fn next(&mut self) -> Result<u8, Error>;

    // fn buffer(&mut self) -> &[u8];

    /// Read n bytes
    ///
    /// Prefered to use this when the size is small and can be stack allocated
    fn read_const_bytes<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let mut buf = [0u8; N];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn read_bytes(&mut self, n: usize) -> Result<Vec<u8>, Error> {
        let mut buf = vec![0u8; n];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }
    
    fn read_item_bytes_with_format_code(&mut self) -> Result<Vec<u8>, Error> {
        use crate::format::Category;
        let code_byte = self.next()?;
        let code: EncodingCodes = code_byte.try_into()?;
        let mut vec = match code.try_into()? {
            Category::Fixed(w) => {
                let mut buf = vec![0u8; 1 + w as usize];
                self.read_exact(&mut buf[1..])?;
                buf
            },
            Category::Encoded(w) => {
                match w {
                    EncodedWidth::Zero => return Ok(vec![code_byte]),
                    EncodedWidth::One => {
                        let len = self.next()?;
                        let mut buf = vec![0u8; 1 + 1 + len as usize];
                        self.read_exact(&mut buf[2..])?;
                        buf[1] = len;
                        buf
                    },
                    EncodedWidth::Four => {
                        let len_bytes = self.read_const_bytes()?;
                        let len = u32::from_be_bytes(len_bytes);
                        let mut buf = vec![0u8; 1 + 4 + len as usize];
                        self.read_exact(&mut buf[5..])?;
                        (&mut buf[1..5]).copy_from_slice(&len_bytes);
                        buf
                    }
                }
            },
            // Category::Compound(w) => {
            //     match w {
            //         CompoundWidth::Zero => {
            //             return Ok(vec![code_byte])
            //         },
            //         CompoundWidth::One => {
            //             let len = self.next()?;
            //             let mut buf = vec![0u8; 1 + 1 + len as usize];
            //             self.read_exact(&mut buf[2..])?;
            //             buf[1] = len;
            //             buf
            //         },
            //         CompoundWidth::Four => {
            //             let len_bytes = self.read_const_bytes()?;
            //             let len = u32::from_be_bytes(len_bytes.clone());
            //             let mut buf = vec![0u8; 1 + 4 + len as usize];
            //             self.read_exact(&mut buf[5..])?;
            //             (&mut buf[1..5]).copy_from_slice(&len_bytes);
            //             buf
            //         }
            //     }
            // },
            // Category::Array(w) => {
            //     match w {
            //         ArrayWidth::One => {
            //             let len = 
            //         },
            //         ArrayWidth::Four => {

            //         }
            //     }
            // }
        };
        vec[0] = code_byte;
        Ok(vec)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Error>;

    fn forward_read_bytes<V>(&mut self, len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>;

    fn forward_read_str<V>(&mut self, len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>;
}
