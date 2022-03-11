use std::io;

use crate::error::Error;

use super::{private, Read};

pub(crate) struct IoReader<R> {
    // an io reader
    reader: R,
    buf: Vec<u8>,
}

impl<R: io::Read> IoReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            // next_byte: None,
            buf: Vec::new(),
        }
    }

    pub fn pop_first(&mut self) -> Option<u8> {
        match self.buf.is_empty() {
            true => None,
            false => Some(self.buf.remove(0)),
        }
    }

    pub fn fill_buffer(&mut self, len: usize) -> Result<(), Error> {
        let l = self.buf.len();
        if l < len {
            self.buf.resize(len, 0);
            self.reader.read_exact(&mut self.buf[l..])?;
            Ok(())
        } else {
            Ok(())
        }
    }
}

impl<R: io::Read> private::Sealed for IoReader<R> {}

impl<'de, R: io::Read + 'de> Read<'de> for IoReader<R> {
    fn peek(&mut self) -> Result<u8, Error> {
        match self.buf.first() {
            Some(b) => Ok(*b),
            None => {
                let mut buf = [0u8; 1];
                self.reader.read_exact(&mut buf)?;
                self.buf.push(buf[0]);
                Ok(buf[0])
            }
        }
    }

    fn peek_bytes(&mut self, n: usize) -> Result<&[u8], Error> {
        let l = self.buf.len();
        if l < n {
            self.fill_buffer(n)?;
            Ok(&self.buf[..n])
        } else {
            Ok(&self.buf[..n])
        }
    }

    fn next(&mut self) -> Result<u8, Error> {
        match self.pop_first() {
            Some(b) => Ok(b),
            None => {
                let mut buf = [0u8; 1];
                self.reader.read_exact(&mut buf)?;
                Ok(buf[0])
            }
        }
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        let n = buf.len();
        let l = self.buf.len();

        if l < n {
            (&mut buf[..l]).copy_from_slice(&self.buf[..l]);
            self.reader.read_exact(&mut buf[l..])?;
            self.buf.drain(..l);
            Ok(())
        } else {
            buf.copy_from_slice(&self.buf[..n]);
            self.buf.drain(..n);
            Ok(())
        }
    }

    fn forward_read_bytes<V>(&mut self, len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.fill_buffer(len)?;
        visitor.visit_bytes(&self.buf[..len])
    }

    fn forward_read_str<V>(&mut self, len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.fill_buffer(len)?;
        let s = std::str::from_utf8(&self.buf[..len])?;
        visitor.visit_str(s)
    }
}

#[cfg(test)]
mod tests {
    use crate::read::ioread::IoReader;

    use super::Read;

    const SHORT_BUFFER: &[u8] = &[0, 1, 2];
    const LONG_BUFFER: &[u8] = &[
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
    ];

    #[test]
    fn test_peek() {
        let reader = SHORT_BUFFER;
        let mut io_reader = IoReader::new(reader);

        let peek0 = io_reader.peek().expect("Should not return error");
        let peek1 = io_reader.peek().expect("Should not return error");
        let peek2 = io_reader.peek().expect("Should not return error");

        assert_eq!(peek0, reader[0]);
        assert_eq!(peek1, reader[0]);
        assert_eq!(peek2, reader[0]);
    }

    #[test]
    fn test_next() {
        let reader = SHORT_BUFFER;
        let mut io_reader = IoReader::new(reader);

        for i in 0..reader.len() {
            let peek = io_reader.peek().expect("Should not return error");
            let next = io_reader.next().expect("Should not return error");

            assert_eq!(peek, reader[i]);
            assert_eq!(next, reader[i]);
        }

        let peek_none = io_reader.peek();
        let next_none = io_reader.next();

        assert!(peek_none.is_err());
        assert!(next_none.is_err());
    }

    #[test]
    fn test_read_const_bytes_without_peek() {
        let reader = LONG_BUFFER;
        let mut io_reader = IoReader::new(reader);

        // Read first 10 bytes
        const N: usize = 10;
        let bytes = io_reader
            .read_const_bytes::<N>()
            .expect("Should not return error");
        assert_eq!(bytes.len(), N);
        assert_eq!(&bytes[..], &reader[..N]);

        // Read the second bytes
        let bytes = io_reader
            .read_const_bytes::<N>()
            .expect("Should not return error");
        assert_eq!(bytes.len(), N);
        assert_eq!(&bytes[..], &reader[(N)..(2 * N)]);

        // Read None
        let bytes = io_reader.read_const_bytes::<N>();
        assert!(bytes.is_err());
    }

    #[test]
    fn test_incomplete_read_const_bytes_without_peek() {
        let reader = SHORT_BUFFER;
        let mut io_reader = IoReader::new(std::io::Cursor::new(reader));

        // Read first 10 bytes
        const N: usize = 10;
        let bytes = io_reader.read_const_bytes::<N>();
        assert!(bytes.is_err());

        for i in 0..reader.len() {
            let peek = io_reader.peek().expect("Should not return error");
            let next = io_reader.next().expect("Should not return error");

            assert_eq!(peek, reader[i]);
            assert_eq!(next, reader[i]);
        }

        let peek_none = io_reader.peek();
        let next_none = io_reader.next();

        assert!(peek_none.is_err());
        assert!(next_none.is_err());
    }

    #[test]
    fn test_read_const_bytes_after_peek() {
        let reader = LONG_BUFFER;
        let mut io_reader = IoReader::new(reader);

        let peek0 = io_reader.peek().expect("Should not return error");
        assert_eq!(peek0, reader[0]);

        // Read first 10 bytes
        const N: usize = 10;
        let bytes = io_reader
            .read_const_bytes::<N>()
            .expect("Should not return error");
        assert_eq!(bytes.len(), N);
        assert_eq!(&bytes[..], &reader[..N]);

        // Read the second bytes
        let bytes = io_reader
            .read_const_bytes::<N>()
            .expect("Should not return error");
        assert_eq!(bytes.len(), N);
        assert_eq!(&bytes[..], &reader[(N)..(2 * N)]);

        // Read None
        let bytes = io_reader.read_const_bytes::<N>();
        assert!(bytes.is_err());
    }

    #[test]
    fn test_incomplete_read_const_bytes_after_peek() {
        let reader = SHORT_BUFFER;
        let mut io_reader = IoReader::new(std::io::Cursor::new(reader));

        let peek0 = io_reader.peek().expect("Should not return error");
        assert_eq!(peek0, reader[0]);

        // Read first 10 bytes
        const N: usize = 10;
        let bytes = io_reader.read_const_bytes::<N>();
        assert!(bytes.is_err());

        for i in 0..reader.len() {
            let peek = io_reader.peek().expect("Should not return error");
            let next = io_reader.next().expect("Should not return error");

            assert_eq!(peek, reader[i]);
            assert_eq!(next, reader[i]);
        }

        let peek_err = io_reader.peek();
        let next_err = io_reader.next();

        assert!(peek_err.is_err());
        assert!(next_err.is_err());
    }

    #[test]
    fn test_peek_bytes() {
        let mut reader = IoReader::new(SHORT_BUFFER);
        let peek0 = reader.peek_bytes(2).unwrap().to_vec();
        let peek1 = reader.peek_bytes(2).unwrap();

        assert_eq!(peek0, &SHORT_BUFFER[..2]);
        assert_eq!(peek1, &SHORT_BUFFER[..2]);
    }
}
