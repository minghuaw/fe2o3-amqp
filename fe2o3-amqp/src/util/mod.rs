//! Common utilities

use bytes::Buf;
use fe2o3_amqp_types::definitions::DeliveryNumber;
use fe2o3_amqp_types::messaging::DeliveryState;
use futures_util::Future;
use serde_amqp::read::{IoReader, SliceReader};
use std::io;
use std::ops::Deref;
use std::slice::Iter;
use std::{pin::Pin, task::Poll, time::Duration};

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

pub(crate) trait Instant {
    fn now() -> Self;

    fn add(self, duration: Duration) -> Self;
}

pub(crate) trait Delay: Future {
    type Instant: Instant;

    fn from_duration(duration: Duration) -> Self;
    fn reset_at(self: Pin<&mut Self>, at: Self::Instant);
}

cfg_not_wasm32! {
    impl Instant for tokio::time::Instant {
        fn now() -> Self {
            tokio::time::Instant::now()
        }

        fn add(self, duration: Duration) -> Self {
            self + duration
        }
    }

    type DelayImpl = tokio::time::Sleep;

    impl Delay for tokio::time::Sleep {
        type Instant = tokio::time::Instant;

        fn from_duration(duration: Duration) -> Self {
            tokio::time::sleep(duration)
        }

        fn reset_at(self: Pin<&mut Self>, at: Self::Instant) {
            self.reset(at);
        }
    }
}

cfg_wasm32! {
    impl Instant for wasmtimer::std::Instant {
        fn now() -> Self {
            wasmtimer::std::Instant::now()
        }

        fn add(self, duration: Duration) -> Self {
            self + duration
        }
    }

    type DelayImpl = wasmtimer::tokio::Delay;

    impl Delay for wasmtimer::tokio::Delay {
        type Instant = wasmtimer::std::Instant;

        fn from_duration(duration: Duration) -> Self {
            wasmtimer::tokio::Delay::new(duration)
        }

        fn reset_at(self: Pin<&mut Self>, at: Self::Instant) {
            self.reset(at);
        }
    }
}

#[derive(Debug)]
pub(crate) struct IdleTimeout<T = DelayImpl> 
where 
    T: Delay
{
    delay: Pin<Box<T>>,
    duration: Duration,
}

impl<T> IdleTimeout<T> 
where 
    T: Delay,
{
    pub fn new(duration: Duration) -> Self {
        let delay = Box::pin(T::from_duration(duration));
        Self { delay, duration }
    }

    pub fn reset(&mut self) {
        let now = T::Instant::now();
        let next = now.add(self.duration);
        self.delay.as_mut().reset_at(next);
    }
}

impl Future for IdleTimeout {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let delay = Pin::new(&mut self.delay);
        delay.poll(cx)
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

cfg_acceptor! {
    /// Shared type state for builders
    #[derive(Debug)]
    pub struct Initialized {}
}

pub(crate) trait AsByteIterator {
    type IterImpl<'i>: Iterator<Item = &'i u8> + ExactSizeIterator + DoubleEndedIterator
    where
        Self: 'i;

    fn as_byte_iterator(&self) -> Self::IterImpl<'_>;
}

pub(crate) trait IntoReader<'a> {
    type Reader: serde_amqp::read::Read<'a>;

    fn into_reader(self) -> Self::Reader;
}

impl<'a> IntoReader<'a> for &'a Payload {
    type Reader = SliceReader<'a>;

    fn into_reader(self) -> Self::Reader {
        SliceReader::new(self)
    }
}

impl AsByteIterator for Payload {
    type IterImpl<'i>
        = std::slice::Iter<'i, u8>
    where
        Self: 'i;

    fn as_byte_iterator(&self) -> Self::IterImpl<'_> {
        self.iter()
    }
}

impl AsByteIterator for &Payload {
    type IterImpl<'i>
        = std::slice::Iter<'i, u8>
    where
        Self: 'i;

    fn as_byte_iterator(&self) -> Self::IterImpl<'_> {
        self.iter()
    }
}

impl AsByteIterator for &[u8] {
    type IterImpl<'i>
        = std::slice::Iter<'i, u8>
    where
        Self: 'i;

    fn as_byte_iterator(&self) -> Self::IterImpl<'_> {
        self.iter()
    }
}

impl AsByteIterator for Vec<u8> {
    type IterImpl<'i>
        = std::slice::Iter<'i, u8>
    where
        Self: 'i;

    fn as_byte_iterator(&self) -> Self::IterImpl<'_> {
        self.iter()
    }
}

impl IntoReader<'static> for Vec<Payload> {
    type Reader = IoReader<ByteReader<Payload>>;

    fn into_reader(self) -> Self::Reader {
        IoReader::new(ByteReader { inner: self })
    }
}

impl AsByteIterator for Vec<Payload> {
    type IterImpl<'i>
        = ByteReaderIter<'i>
    where
        Self: 'i;

    fn as_byte_iterator(&self) -> Self::IterImpl<'_> {
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

impl ExactSizeIterator for ByteReaderIter<'_> {
    fn len(&self) -> usize {
        self.inner.iter().map(|iter| iter.len()).sum()
    }
}

impl DoubleEndedIterator for ByteReaderIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner
            .iter_mut()
            .flat_map(|iter| iter.next_back())
            .next_back()
    }
}

pub(crate) trait AsDeliveryState {
    fn as_delivery_state(&self) -> &Option<DeliveryState>;
}

impl AsDeliveryState for Option<DeliveryState> {
    fn as_delivery_state(&self) -> &Option<DeliveryState> {
        self
    }
}

/// A private zero-sized type that makes struct not constructable by users
#[derive(Debug, Clone)]
pub(crate) struct Sealed {}

pub(crate) fn is_consecutive(left: &DeliveryNumber, right: &DeliveryNumber) -> bool {
    // Assume ascending order
    right - left == 1
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use serde_amqp::read::Read;

    use super::{AsByteIterator, IntoReader};

    #[test]
    fn test_multiple_payload_reader() {
        let b0 = Bytes::from(vec![1, 2, 3]);
        let b1 = Bytes::from(vec![4, 5, 6, 7, 8]);
        let b2 = Bytes::from(vec![9]);

        let v = vec![b0, b1, b2];
        let mut reader = v.into_reader();

        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(buf, [1u8]);

        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(buf, [2u8]);

        let mut buf = [0u8; 3];
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(buf, [3, 4, 5]);

        let mut buf = [0u8; 10];
        let res = reader.read_exact(&mut buf);
        assert!(res.is_err());
    }

    #[test]
    fn test_multiple_payload_iter() {
        let b0 = Bytes::from(vec![1, 2, 3]);
        let b1 = Bytes::from(vec![4, 5, 6, 7, 8]);
        let b2 = Bytes::from(vec![9]);

        let v = vec![b0, b1, b2];
        let iter = v.as_byte_iterator();
        assert_eq!(iter.len(), 9);

        let forward: Vec<u8> = iter.copied().collect();
        assert_eq!(forward, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);

        let iter = v.as_byte_iterator();
        let reverse: Vec<u8> = iter.rev().copied().collect();
        assert_eq!(reverse, vec![9, 8, 7, 6, 5, 4, 3, 2, 1]);
    }
}
