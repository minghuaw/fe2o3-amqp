//! Custom `Read` trait

use std::io;

use crate::error::Error;

mod ioread;
pub use ioread::*;

mod sliceread;
pub use sliceread::*;

mod private {
    pub trait Sealed {}
}

/// A custom Read trait for internal use
pub trait Read<'de>: private::Sealed {
    /// Peek the next byte without consuming
    fn peek(&mut self) -> Option<u8>;

    /// Read the next byte
    fn next(&mut self) -> Result<Option<u8>, io::Error>;

    /// Read n bytes
    ///
    /// Prefered to use this when the size is small and can be stack allocated
    fn read_const_bytes<const N: usize>(&mut self) -> Result<[u8; N], io::Error> {
        let mut buf = [0u8; N];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Peek `n` number of bytes without consuming
    fn peek_bytes(&mut self, n: usize) -> Result<Option<&[u8]>, io::Error>;

    /// Consuming `n` number of bytes
    fn read_bytes(&mut self, n: usize) -> Result<Vec<u8>, io::Error> {
        let mut buf = vec![0u8; n];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Read to buffer
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), io::Error>;

    /// Forward bytes to visitor
    fn forward_read_bytes<V>(&mut self, len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>;

    /// Forward str to visitor
    fn forward_read_str<V>(&mut self, len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>;
}
