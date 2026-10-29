use std::io;
use std::io::Read as _;

use crate::error::Error;

use super::{private, read_described_bytes, read_primitive_bytes_or_else, Read};

/// Maximum capacity reserved up front from a wire-declared length. Larger
/// values still grow incrementally; this only avoids the small-allocation
/// staircase for values that are actually present.
const MAX_PREALLOC: usize = 64 * 1024;

#[inline]
fn prealloc_capacity(len: usize) -> usize {
    len.min(MAX_PREALLOC)
}

/// A reader for IO stream
#[derive(Debug)]
pub struct IoReader<R> {
    // an io reader
    reader: R,
    buf: Vec<u8>,
    consumed: usize,
}

impl<R: io::Read> IoReader<R> {
    /// Creates a new reader over IO stream
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            buf: Vec::new(),
            consumed: 0,
        }
    }

    /// Pop the first byte
    pub fn pop_first(&mut self) -> Option<u8> {
        match self.buf.is_empty() {
            true => None,
            false => Some(self.buf.remove(0)),
        }
    }

    /// Fill the internal buffer with the given length.
    ///
    /// The buffer grows incrementally while reading. A declared length that is
    /// much larger than the input available must not cause a proportional
    /// allocation before the input is exhausted.
    pub fn fill_buffer(&mut self, len: usize) -> Result<(), io::Error> {
        if self.buf.len() >= len {
            return Ok(());
        }

        // `read_to_end` appends into the vector's spare capacity, so the
        // buffer grows only as bytes actually arrive and we avoid both a
        // declared-length resize and an intermediate stack copy. Reserve only
        // a bounded amount up front so a malformed length cannot force a huge
        // allocation.
        self.buf.reserve(prealloc_capacity(len - self.buf.len()));
        let mut limited = (&mut self.reader).take((len - self.buf.len()) as u64);
        limited.read_to_end(&mut self.buf)?;

        if self.buf.len() < len {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "declared value length exceeds available bytes",
            ));
        }

        Ok(())
    }
}

impl<R: io::Read> private::Sealed for IoReader<R> {}

impl<'de, R: io::Read + 'de> Read<'de> for IoReader<R> {
    fn peek(&mut self) -> Option<u8> {
        match self.buf.first() {
            Some(b) => Some(*b),
            None => {
                let mut buf = [0u8; 1];
                match self.reader.read_exact(&mut buf) {
                    Ok(_) => {
                        self.buf.push(buf[0]);
                        Some(buf[0])
                    }
                    Err(_) => None,
                }
            }
        }
    }

    fn bytes_consumed(&self) -> usize {
        self.consumed
    }

    fn peek_bytes(&mut self, n: usize) -> Result<Option<&[u8]>, io::Error> {
        let l = self.buf.len();
        if l < n {
            self.fill_buffer(n)?;
            Ok(Some(&self.buf[..n]))
        } else {
            Ok(Some(&self.buf[..n]))
        }
    }

    fn next(&mut self) -> Result<Option<u8>, io::Error> {
        match self.pop_first() {
            Some(b) => {
                self.consumed += 1;
                Ok(Some(b))
            }
            None => {
                let mut buf = [0u8; 1];
                self.reader.read_exact(&mut buf)?;
                self.consumed += 1;
                Ok(Some(buf[0]))
            }
        }
    }

    fn read_bytes(&mut self, n: usize) -> Result<Vec<u8>, io::Error> {
        if self.buf.len() >= n {
            let bytes = self.buf.drain(..n).collect();
            self.consumed += n;
            return Ok(bytes);
        }

        // Take any already-buffered prefix, then read the rest directly into
        // the returned vector. This keeps the common path at one copy while
        // still avoiding an allocation proportional to a declared length that
        // is not backed by input.
        let mut bytes = Vec::with_capacity(prealloc_capacity(n));
        bytes.append(&mut self.buf);
        let remaining = n - bytes.len();
        let mut limited = (&mut self.reader).take(remaining as u64);
        limited.read_to_end(&mut bytes)?;

        if bytes.len() < n {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "declared value length exceeds available bytes",
            ));
        }

        self.consumed += n;
        Ok(bytes)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), io::Error> {
        let n = buf.len();
        let l = self.buf.len();

        if l < n {
            (buf[..l]).copy_from_slice(&self.buf[..l]);
            let result = self.reader.read_exact(&mut buf[l..]);
            // drain the buffer even if the read fails
            self.buf.drain(..l);
            if result.is_ok() {
                self.consumed += n;
            }
            result
        } else {
            buf.copy_from_slice(&self.buf[..n]);
            self.buf.drain(..n);
            self.consumed += n;
            Ok(())
        }
    }

    fn forward_read_bytes_with_hint<V>(&mut self, len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.fill_buffer(len)?;
        let result = visitor.visit_bytes(&self.buf[..len]);
        self.buf.drain(..len);
        self.consumed += len;
        result
    }

    fn forward_read_byte_buf<V>(&mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let bytes = read_primitive_bytes_or_else(self, read_described_bytes)?;
        visitor.visit_bytes(&bytes)
    }

    fn forward_read_str<V>(&mut self, len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.fill_buffer(len)?;
        let s = std::str::from_utf8(&self.buf[..len])?;
        let result = visitor.visit_str(s);
        self.buf.drain(..len);
        self.consumed += len;
        result
    }
}

#[allow(clippy::all)]
#[cfg(test)]
mod tests {
    use bytes::Buf;

    use crate::read::ioread::IoReader;

    use super::{prealloc_capacity, Read, MAX_PREALLOC};

    const SHORT_BUFFER: &[u8] = &[0, 1, 2];
    const LONG_BUFFER: &[u8] = &[
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
    ];

    /// A reader that returns at most `chunk` bytes per `read` call and counts
    /// how often it was polled.
    struct ChunkedReader {
        data: &'static [u8],
        pos: usize,
        chunk: usize,
        reads: usize,
    }

    impl ChunkedReader {
        fn new(data: &'static [u8], chunk: usize) -> Self {
            Self {
                data,
                pos: 0,
                chunk,
                reads: 0,
            }
        }

        fn reads(&self) -> usize {
            self.reads
        }
    }

    impl std::io::Read for ChunkedReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.reads += 1;
            let remaining = &self.data[self.pos..];
            let n = buf.len().min(self.chunk).min(remaining.len());
            buf[..n].copy_from_slice(&remaining[..n]);
            self.pos += n;
            Ok(n)
        }
    }

    #[test]
    fn test_peek() {
        let reader = SHORT_BUFFER;
        let mut io_reader = IoReader::new(reader);

        let peek0 = io_reader.peek().unwrap();
        let peek1 = io_reader.peek().unwrap();
        let peek2 = io_reader.peek().unwrap();

        assert_eq!(peek0, reader[0]);
        assert_eq!(peek1, reader[0]);
        assert_eq!(peek2, reader[0]);
    }

    #[test]
    fn test_next() {
        let reader = SHORT_BUFFER;
        let mut io_reader = IoReader::new(reader);

        for i in 0..reader.len() {
            let peek = io_reader.peek().unwrap();
            let next = io_reader.next().unwrap().unwrap();

            assert_eq!(peek, reader[i]);
            assert_eq!(next, reader[i]);
        }

        let peek_none = io_reader.peek();
        let next_none = io_reader.next();

        assert!(peek_none.is_none());
        assert!(next_none.is_err() || matches!(next_none, Ok(None)));
    }

    #[test]
    fn test_read_bytes_rejects_length_larger_than_input() {
        let mut reader = IoReader::new(std::io::Cursor::new(SHORT_BUFFER));

        // The old implementation allocated `n` bytes before reading. Using a
        // deliberately enormous length makes that failure mode obvious.
        assert!(reader.read_bytes(u32::MAX as usize).is_err());
    }

    #[test]
    fn test_fill_buffer_rejects_length_larger_than_input() {
        let mut reader = IoReader::new(std::io::Cursor::new(SHORT_BUFFER));

        assert!(reader.fill_buffer(1_000_000).is_err());
        assert!(reader.peek_bytes(1_000_000).is_err());
    }

    #[test]
    fn test_prealloc_capacity_is_bounded() {
        assert_eq!(prealloc_capacity(0), 0);
        assert_eq!(prealloc_capacity(1024), 1024);
        assert_eq!(prealloc_capacity(MAX_PREALLOC), MAX_PREALLOC);
        assert_eq!(prealloc_capacity(usize::MAX), MAX_PREALLOC);
    }

    #[test]
    fn test_read_bytes_exact_length_and_consumed() {
        let mut reader = IoReader::new(std::io::Cursor::new(b"abcdef"));

        let bytes = reader.read_bytes(3).unwrap();
        assert_eq!(&bytes, b"abc");
        assert_eq!(reader.bytes_consumed(), 3);

        let bytes = reader.read_bytes(3).unwrap();
        assert_eq!(&bytes, b"def");
        assert_eq!(reader.bytes_consumed(), 6);

        assert!(reader.read_bytes(1).is_err());
        assert_eq!(reader.bytes_consumed(), 6);
    }

    #[test]
    fn test_read_bytes_uses_prefetched_prefix() {
        let mut reader = IoReader::new(std::io::Cursor::new(b"abcdef"));

        assert_eq!(reader.peek_bytes(5).unwrap().unwrap(), b"abcde");

        // n < buffered: consume only two bytes and leave the rest buffered.
        let bytes = reader.read_bytes(2).unwrap();
        assert_eq!(&bytes, b"ab");
        assert_eq!(reader.bytes_consumed(), 2);
        assert_eq!(reader.peek().unwrap(), b'c');

        // n == buffered remainder: drain the rest of the peeked prefix.
        let bytes = reader.read_bytes(3).unwrap();
        assert_eq!(&bytes, b"cde");
        assert_eq!(reader.bytes_consumed(), 5);

        // The next read comes from the underlying reader.
        let bytes = reader.read_bytes(1).unwrap();
        assert_eq!(&bytes, b"f");
        assert_eq!(reader.bytes_consumed(), 6);
    }

    #[test]
    fn test_read_bytes_combines_prefetched_prefix_with_more_input() {
        let mut reader = IoReader::new(std::io::Cursor::new(b"abcdef"));

        assert_eq!(reader.peek_bytes(3).unwrap().unwrap(), b"abc");

        // n > buffered: drain the prefix and read the rest directly.
        let bytes = reader.read_bytes(5).unwrap();
        assert_eq!(&bytes, b"abcde");
        assert_eq!(reader.bytes_consumed(), 5);

        let bytes = reader.read_bytes(1).unwrap();
        assert_eq!(&bytes, b"f");
    }

    #[test]
    fn test_read_bytes_zero_length_preserves_buffer() {
        let mut reader = IoReader::new(std::io::Cursor::new(b"abc"));

        assert_eq!(reader.peek_bytes(3).unwrap().unwrap(), b"abc");

        let bytes = reader.read_bytes(0).unwrap();
        assert!(bytes.is_empty());
        assert_eq!(reader.bytes_consumed(), 0);
        assert_eq!(reader.peek().unwrap(), b'a');

        let bytes = reader.read_bytes(3).unwrap();
        assert_eq!(&bytes, b"abc");
        assert_eq!(reader.bytes_consumed(), 3);
    }

    #[test]
    fn test_fill_buffer_early_return_does_not_read_again() {
        let mut reader = IoReader::new(ChunkedReader::new(b"abc", 1));

        assert_eq!(reader.peek_bytes(2).unwrap().unwrap(), b"ab");
        let reads = reader.reader.reads();

        assert!(reader.fill_buffer(1).is_ok());
        assert_eq!(reader.reader.reads(), reads);
        assert_eq!(reader.peek_bytes(2).unwrap().unwrap(), b"ab");
    }

    #[test]
    fn test_read_bytes_handles_partial_underlying_reads() {
        let mut reader = IoReader::new(ChunkedReader::new(b"abcdef", 2));

        let bytes = reader.read_bytes(6).unwrap();
        assert_eq!(&bytes, b"abcdef");
        assert_eq!(reader.bytes_consumed(), 6);

        let mut reader = IoReader::new(ChunkedReader::new(b"abcde", 2));
        assert!(reader.read_bytes(6).is_err());
    }

    #[test]
    fn test_fill_buffer_handles_partial_underlying_reads() {
        let mut reader = IoReader::new(ChunkedReader::new(b"abcde", 2));

        assert_eq!(reader.peek_bytes(5).unwrap().unwrap(), b"abcde");
        assert!(reader.reader.reads() > 1);
    }

    #[test]
    fn test_read_const_bytes_without_peek() {
        let reader = LONG_BUFFER;
        let mut io_reader = IoReader::new(reader);

        // Read first 10 bytes
        const N: usize = 10;
        let bytes = io_reader.read_const_bytes::<N>().unwrap();
        assert_eq!(bytes.len(), N);
        assert_eq!(&bytes[..], &reader[..N]);

        // Read the second bytes
        let bytes = io_reader.read_const_bytes::<N>().unwrap();
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

        let peek_none = io_reader.peek();
        let next_none = io_reader.next();

        assert!(peek_none.is_none());
        assert!(matches!(next_none, Ok(None)) || next_none.is_err());
    }

    #[test]
    fn test_read_const_bytes_after_peek() {
        let reader = LONG_BUFFER;
        let mut io_reader = IoReader::new(reader);

        let peek0 = io_reader.peek().unwrap();
        assert_eq!(peek0, reader[0]);

        // Read first 10 bytes
        const N: usize = 10;
        let bytes = io_reader.read_const_bytes::<N>().unwrap();
        assert_eq!(bytes.len(), N);
        assert_eq!(&bytes[..], &reader[..N]);

        // Read the second bytes
        let bytes = io_reader.read_const_bytes::<N>().unwrap();
        assert_eq!(bytes.len(), N);
        assert_eq!(&bytes[..], &reader[(N)..(2 * N)]);

        // Read None
        let bytes = io_reader.read_const_bytes::<N>();
        assert!(bytes.is_err());
    }

    #[test]
    fn test_incomplete_read_const_bytes_after_peek() {
        let b = bytes::Bytes::from_static(SHORT_BUFFER);
        let mut io_reader = IoReader::new(b.reader());

        let peek0 = io_reader.peek().unwrap();
        assert_eq!(peek0, SHORT_BUFFER[0]);

        // Read first 10 bytes
        const N: usize = 10;
        let bytes = io_reader.read_const_bytes::<N>();
        assert!(bytes.is_err());

        let peek_err = io_reader.peek();
        let next_err = io_reader.next();

        assert!(peek_err.is_none());
        assert!(matches!(next_err, Ok(None)) || next_err.is_err());
    }

    #[test]
    fn test_peek_bytes() {
        let mut reader = IoReader::new(SHORT_BUFFER);
        let peek0 = reader.peek_bytes(2).unwrap().unwrap().to_vec();
        let peek1 = reader.peek_bytes(2).unwrap().unwrap();

        assert_eq!(peek0, &SHORT_BUFFER[..2]);
        assert_eq!(peek1, &SHORT_BUFFER[..2]);
    }
}
