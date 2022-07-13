//! Common utilities

use bytes::Bytes;
use futures_util::Future;
use std::ops::Deref;
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
    type IterImpl: Iterator<Item = u8> + ExactSizeIterator + DoubleEndedIterator;

    fn as_byte_iterator(&'a self) -> Self::IterImpl;
}

/// This is used as a buffer for multi-frame delivery
#[derive(Debug)]
pub(crate) struct BytesReader<T>(pub T);

impl std::io::Read for BytesReader<Vec<Payload>> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        todo!()
    }
}

impl<'a> AsByteIterator<'a> for BytesReader<Vec<Payload>> {
    type IterImpl = BytesReaderIter<Vec<&'a [u8]>>;
    
    fn as_byte_iterator(&self) -> Self::IterImpl {
        todo!()
    }
}

impl std::io::Read for BytesReader<Payload> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        todo!()
    }
}

impl<'a> AsByteIterator<'a> for BytesReader<Payload> {
    type IterImpl = BytesReaderIter<&'a [u8]>;
    
    fn as_byte_iterator(&self) -> Self::IterImpl {
        todo!()
    }
}

pub(crate) struct BytesReaderIter<T>(pub T);

/// 
// pub(crate) struct BytesReaderIter<'a>(pub Vec<&'a [u8]>);

impl<'a> Iterator for BytesReaderIter<Vec<&'a [u8]>> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

impl<'a> ExactSizeIterator for BytesReaderIter<Vec<&'a [u8]>> {
    fn len(&self) -> usize {
        todo!()
    }
}

impl<'a> DoubleEndedIterator for BytesReaderIter<Vec<&'a [u8]>> {
    fn next_back(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

impl<'a> Iterator for BytesReaderIter<&'a [u8]> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

impl<'a> ExactSizeIterator for BytesReaderIter<&'a [u8]> {
    fn len(&self) -> usize {
        todo!()
    }
}

impl<'a> DoubleEndedIterator for BytesReaderIter<&'a [u8]> {
    fn next_back(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
