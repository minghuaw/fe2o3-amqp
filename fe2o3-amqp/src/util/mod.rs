//! Common utilities

use bytes::{buf, Buf};
use fe2o3_amqp_types::definitions::DeliveryNumber;
use fe2o3_amqp_types::messaging::DeliveryState;
use futures_util::Future;
use std::io;
use std::ops::Deref;
use std::slice::Iter;
use std::{pin::Pin, task::Poll, time::Duration};
use tokio::time::Instant;
use tokio::time::Sleep;

mod consumer;
mod producer;
pub use consumer::*;
pub use producer::*;

use crate::Payload;

#[derive(Debug)]
pub(crate) enum Running {
    Continue,
    Stop,
}

#[derive(Debug)]
pub(crate) struct IdleTimeout {
    delay: Pin<Box<Sleep>>,
    duration: Duration,
}

impl IdleTimeout {
    pub fn new(duration: Duration) -> Self {
        let delay = Box::pin(tokio::time::sleep(duration));
        Self { delay, duration }
    }

    pub fn reset(&mut self) {
        let now = Instant::now();
        let next = now + self.duration;
        self.delay.as_mut().reset(next);
    }
}

impl Future for IdleTimeout {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        self.delay.as_mut().poll(cx)
        // .map(|_| IdleTimeoutElapsed {  } )
    }
}

/// An custom type to make a field immutable to
/// prevent accidental mutations
#[derive(Debug)]
pub(crate) struct Constant<T> {
    value: T,
}

impl<T> Constant<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }

    pub fn value(&self) -> &T {
        &self.value
    }
}

impl<T> Deref for Constant<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

/// Shared type state for builders
#[derive(Debug)]
pub struct Uninitialized {}

/// Shared type state for builders
#[derive(Debug)]
pub struct Initialized {}

pub(crate) trait AsByteIterator<'a> {
    type IterImpl: Iterator<Item = &'a u8> + ExactSizeIterator + DoubleEndedIterator;

    fn as_byte_iterator(&'a self) -> Self::IterImpl;
}

pub(crate) trait IntoReader {
    type Reader: io::Read;

    fn into_reader(self) -> Self::Reader;
}

impl IntoReader for Payload {
    type Reader = buf::Reader<Payload>;

    fn into_reader(self) -> Self::Reader {
        self.reader()
    }
}

impl<'a> AsByteIterator<'a> for Payload {
    type IterImpl = std::slice::Iter<'a, u8>;

    fn as_byte_iterator(&'a self) -> Self::IterImpl {
        self.iter()
    }
}

impl<'a> AsByteIterator<'a> for &[u8] {
    type IterImpl = std::slice::Iter<'a, u8>;

    fn as_byte_iterator(&'a self) -> Self::IterImpl {
        self.iter()
    }
}

impl<'a> AsByteIterator<'a> for Vec<u8> {
    type IterImpl = std::slice::Iter<'a, u8>;

    fn as_byte_iterator(&'a self) -> Self::IterImpl {
        self.iter()
    }
}

impl IntoReader for Vec<Payload> {
    type Reader = ByteReader<Payload>;

    fn into_reader(self) -> Self::Reader {
        ByteReader { inner: self }
    }
}

impl<'a> AsByteIterator<'a> for Vec<Payload> {
    type IterImpl = ByteReaderIter<'a>;

    fn as_byte_iterator(&'a self) -> Self::IterImpl {
        ByteReaderIter {
            inner: self.iter().map(|p| p.iter()).collect(),
        }
    }
}

pub(crate) struct ByteReader<T> {
    inner: Vec<T>,
}

impl io::Read for ByteReader<Payload> {
    fn read(&mut self, dst: &mut [u8]) -> io::Result<usize> {
        let mut nbytes_read = 0;

        for payload in self.inner.iter_mut() {
            if payload.remaining() >= dst.len() - nbytes_read {
                let mut partial = payload.split_to(dst.len() - nbytes_read);
                Buf::copy_to_slice(&mut partial, &mut dst[nbytes_read..]);
                nbytes_read = dst.len();
                break;
            } else if payload.remaining() < dst.len() {
                let remaining = payload.remaining();
                Buf::copy_to_slice(payload, &mut dst[nbytes_read..nbytes_read + remaining]);
                nbytes_read += remaining;
            }
        }

        Ok(nbytes_read)
    }
}

pub(crate) struct ByteReaderIter<'a> {
    pub inner: Vec<Iter<'a, u8>>,
}

impl<'a> Iterator for ByteReaderIter<'a> {
    type Item = &'a u8;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.iter_mut().flat_map(|iter| iter.next()).next()
    }
}

impl<'a> ExactSizeIterator for ByteReaderIter<'a> {
    fn len(&self) -> usize {
        self.inner.iter().map(|iter| iter.len()).sum()
    }
}

impl<'a> DoubleEndedIterator for ByteReaderIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner
            .iter_mut()
            .flat_map(|iter| iter.next_back())
            .next_back()
    }
}

pub(crate) trait AsDeliveryState {
    fn as_delivery_state(&self) -> &Option<DeliveryState>;

    fn as_delivery_state_mut(&mut self) -> &mut Option<DeliveryState>;
}

impl AsDeliveryState for Option<DeliveryState> {
    fn as_delivery_state(&self) -> &Option<DeliveryState> {
        self
    }

    fn as_delivery_state_mut(&mut self) -> &mut Option<DeliveryState> {
        self
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use bytes::{Buf, Bytes};

    use super::{AsByteIterator, IntoReader};

    #[test]
    fn test_multile_payload_reader() {
        let b0 = Bytes::from(vec![1, 2, 3]);
        let b1 = Bytes::from(vec![4, 5, 6, 7, 8]);
        let b2 = Bytes::from(vec![9]);

        let v = vec![b0, b1, b2];
        let mut reader = v.into_reader();

        let mut buf = [0u8; 1];
        let nread = reader.read(&mut buf).unwrap();
        assert_eq!(nread, 1);
        assert_eq!(buf, [1u8]);

        let mut buf = [0u8; 1];
        let nread = reader.read(&mut buf).unwrap();
        assert_eq!(nread, 1);
        assert_eq!(buf, [2u8]);

        let mut buf = [0u8; 3];
        let nread = reader.read(&mut buf).unwrap();
        assert_eq!(nread, 3);
        assert_eq!(buf, [3, 4, 5]);

        let mut buf = [0u8; 10];
        let nread = reader.read(&mut buf).unwrap();
        assert_eq!(nread, 4);
        assert_eq!(buf, [6, 7, 8, 9, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_regular_read() {
        let src = &[1u8, 2, 3, 4];
        let mut dst = [0u8; 6];

        let nread = src.reader().read(&mut dst).unwrap();
        assert_eq!(nread, 4);
        assert_eq!(dst, [1, 2, 3, 4, 0, 0]);
    }

    #[test]
    fn test_multiply_payload_iter() {
        let b0 = Bytes::from(vec![1, 2, 3]);
        let b1 = Bytes::from(vec![4, 5, 6, 7, 8]);
        let b2 = Bytes::from(vec![9]);

        let v = vec![b0, b1, b2];
        let iter = v.as_byte_iterator();
        assert_eq!(iter.len(), 9);

        let forward: Vec<u8> = iter.map(|e| *e).collect();
        assert_eq!(forward, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);

        let iter = v.as_byte_iterator();
        let reverse: Vec<u8> = iter.rev().map(|e| *e).collect();
        assert_eq!(reverse, vec![9, 8, 7, 6, 5, 4, 3, 2, 1]);
    }
}

pub(crate) fn is_consecutive(left: &DeliveryNumber, right: &DeliveryNumber) -> bool {
    // Assume ascending order
    right - left == 1
}
