use std::io;

use crate::error::Error;

use super::{private, Read};

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

    pub fn get_byte_slice(&mut self, n: usize) -> Result<&'s [u8], Error> {
        if self.slice.len() < n {
            return Err(Self::unexpected_eof(""))
        }
        let (read_slice, remaining) = self.slice.split_at(n);
        self.slice = remaining;
        Ok(read_slice)
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
                // let (next, remaining) = self.slice.split_at(1);
                // self.slice = remaining;
                // Ok(next[0])
                let buf = self.get_byte_slice(1)?;
                Ok(buf[0])
            }
        }
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        let n = buf.len();
        
        if self.slice.len() < n {
            Err(Self::unexpected_eof(""))
        } else {
            // let (read_slice, remaining) = self.slice.split_at(n);
            // self.slice = remaining;
            // Ok(())
            let read_slice = self.get_byte_slice(n)?;
            buf.copy_from_slice(read_slice);
            Ok(())
        }
    }

    fn forward_read_bytes<V>(&mut self, len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'s> 
    {
        visitor.visit_borrowed_bytes(self.get_byte_slice(len)?)
    }

    fn forward_read_str<V>(&mut self, len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'s> 
    {
        let str_slice = std::str::from_utf8(self.get_byte_slice(len)?)?;
        visitor.visit_borrowed_str(str_slice)
    }
}
#[cfg(test)]
mod tests {
    use super::{Read, SliceReader};

    const SHORT_BUFFER: &[u8] = &[0, 1, 2];
    const LONG_BUFFER: &[u8] = &[
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
    ];

    #[test]
    fn test_peek() {
        let slice = SHORT_BUFFER;
        let mut reader = SliceReader::new(slice);

        let peek0 = reader.peek().expect("Should not return error");
        let peek1 = reader.peek().expect("Should not return error");
        let peek2 = reader.peek().expect("Should not return error");

        assert_eq!(peek0, slice[0]);
        assert_eq!(peek1, slice[0]);
        assert_eq!(peek2, slice[0]);
    }

    #[test]
    fn test_next() {
        let slice = SHORT_BUFFER;
        let mut reader = SliceReader::new(slice);

        for i in 0..slice.len() {
            let peek = reader.peek().expect("Should not return error");
            let next = reader.next().expect("Should not return error");

            assert_eq!(peek, slice[i]);
            assert_eq!(next, slice[i]);
        }

        let peek_none = reader.peek();
        let next_none = reader.next();

        assert!(peek_none.is_err());
        assert!(next_none.is_err());
    }


    #[test]
    fn test_read_const_bytes_without_peek() {
        let slice = LONG_BUFFER;
        let mut reader = SliceReader::new(slice);

        // Read first 10 bytes
        const N: usize = 10;
        let bytes = reader
            .read_const_bytes::<N>()
            .expect("Should not return error");
        assert_eq!(bytes.len(), N);
        assert_eq!(&bytes[..], &slice[..N]);

        // Read the second bytes
        let bytes = reader
            .read_const_bytes::<N>()
            .expect("Should not return error");
        assert_eq!(bytes.len(), N);
        assert_eq!(&bytes[..], &slice[(N)..(2 * N)]);

        // Read None
        let bytes = reader.read_const_bytes::<N>();
        assert!(bytes.is_err());
    }

    #[test]
    fn test_incomplete_read_const_bytes_without_peek() {
        let slice = SHORT_BUFFER;
        let mut reader = SliceReader::new(slice);

        // Read first 10 bytes
        const N: usize = 10;
        let bytes = reader.read_const_bytes::<N>();
        assert!(bytes.is_err());

        for i in 0..slice.len() {
            let peek = reader.peek().expect("Should not return error");
            let next = reader.next().expect("Should not return error");

            assert_eq!(peek, slice[i]);
            assert_eq!(next, slice[i]);
        }

        let peek_none = reader.peek();
        let next_none = reader.next();

        assert!(peek_none.is_err());
        assert!(next_none.is_err());
    }

    #[test]
    fn test_read_const_bytes_after_peek() {
        let slice = LONG_BUFFER;
        let mut reader = SliceReader::new(slice);

        let peek0 = reader.peek().expect("Should not return error");
        assert_eq!(peek0, slice[0]);

        // Read first 10 bytes
        const N: usize = 10;
        let bytes = reader
            .read_const_bytes::<N>()
            .expect("Should not return error");
        assert_eq!(bytes.len(), N);
        assert_eq!(&bytes[..], &slice[..N]);

        // Read the second bytes
        let bytes = reader
            .read_const_bytes::<N>()
            .expect("Should not return error");
        assert_eq!(bytes.len(), N);
        assert_eq!(&bytes[..], &slice[(N)..(2 * N)]);

        // Read None
        let bytes = reader.read_const_bytes::<N>();
        assert!(bytes.is_err());
    }

    #[test]
    fn test_incomplete_read_const_bytes_after_peek() {
        let slice = SHORT_BUFFER;
        let mut reader = SliceReader::new(slice);

        let peek0 = reader.peek().expect("Should not return error");
        assert_eq!(peek0, slice[0]);

        // Read first 10 bytes
        const N: usize = 10;
        let bytes = reader.read_const_bytes::<N>();
        assert!(bytes.is_err());

        for i in 0..slice.len() {
            let peek = reader.peek().expect("Should not return error");
            let next = reader.next().expect("Should not return error");

            // println!("peek {:?}", peek);
            // println!("next {:?}", next);

            assert_eq!(peek, slice[i]);
            assert_eq!(next, slice[i]);
        }

        let peek_err = reader.peek();
        let next_err = reader.next();

        assert!(peek_err.is_err());
        assert!(next_err.is_err());
    }
}