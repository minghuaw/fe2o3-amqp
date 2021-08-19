use std::io;

use crate::error::Error;

use super::{private, Read};

// TODO: add SliceReader
pub struct SliceReader<'s> {
    slice: &'s [u8]
}

impl<'s> SliceReader<'s> {
    pub fn new(slice: &'s [u8]) -> Self {
        Self {
            slice
        }
    }

    pub fn unexpected_eof(msg: &str) -> Error {
        Error::Io(io::Error::new(
            io::ErrorKind::UnexpectedEof, 
            msg
        ))
    }
}

impl<'s> private::Sealed for SliceReader<'s> { }

impl<'s> Read<'s> for SliceReader<'s> {
    fn peek(&mut self) -> Result<u8, Error> {
        match self.slice.first() {
            Some(b) => Ok(*b),
            None => Err(Self::unexpected_eof("")),
        }
    }

    fn next(&mut self) -> Result<u8, Error> {
        match self.slice.len() {
            0 => Err(Self::unexpected_eof("")),
            _ => {
                let (next, remaining) = self.slice.split_at(1);
                self.slice = remaining;
                Ok(next[0])
            }
        }
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        let n = buf.len();
        
        if self.slice.len() < n {
            Err(Self::unexpected_eof(""))
        } else {
            let (read_slice, remaining) = self.slice.split_at(n);
            buf.copy_from_slice(read_slice);
            self.slice = remaining;
            Ok(())
        }
    }

    fn forward_read_bytes<V>(&mut self, len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'s> 
    {
        todo!()
    }

    fn forward_read_str<V>(&mut self, len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'s> 
    {
        todo!()   
    }
}