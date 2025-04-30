//! Implements an asynchronous heartbeat

use std::{
    io,
    task::{Context, Poll},
    time::Duration,
};

use futures_util::Stream;
use pin_project_lite::pin_project;

/// A helper trait to abstract over the `Interval` type used in the heartbeat implementation.
trait Interval: Unpin {
    /// Type of the instant returned by the interval.
    type Instant;

    /// Create a new interval with the given period.
    fn new_with_period(period: Duration) -> Self;

    /// Poll the interval for a tick.
    fn poll_tick(&mut self, cx: &mut Context<'_>) -> Poll<Self::Instant>;
}

cfg_not_wasm32! {
    type IntervalImpl = tokio::time::Interval;

    impl Interval for tokio::time::Interval {
        type Instant = tokio::time::Instant;

        fn new_with_period(period: Duration) -> Self {
            tokio::time::interval(period)
        }

        fn poll_tick(&mut self, cx: &mut Context<'_>) -> Poll<Self::Instant> {
            tokio::time::Interval::poll_tick(self, cx)
        }
    }
}

cfg_wasm32! {
    type IntervalImpl = wasmtimer::tokio::Interval;

    impl Interval for wasmtimer::tokio::Interval {
        type Instant = wasmtimer::std::Instant;

        fn new_with_period(period: Duration) -> Self {
            wasmtimer::tokio::interval(period)
        }

        fn poll_tick(&mut self, cx: &mut Context<'_>) -> Poll<Self::Instant> {
            self.poll_tick(cx)
        }
    }
}

#[derive(Debug)]
struct IntervalStream<T = IntervalImpl> {
    interval: T,
}

impl<T> IntervalStream<T>
where
    T: Interval,
{
    /// Create a new `IntervalStream` with the given period.
    fn new(period: Duration) -> Self {
        Self {
            interval: T::new_with_period(period),
        }
    }
}

impl<T> Stream for IntervalStream<T>
where
    T: Interval,
{
    type Item = T::Instant;

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match this.interval.poll_tick(cx) {
            Poll::Ready(instant) => Poll::Ready(Some(instant)),
            Poll::Pending => Poll::Pending,
        }
    }
}

pin_project! {
    /// A wrapper over an `Option<IntervalStream>` which will never tick ready if the underlying
    /// `Interval` is `None`
    #[derive(Debug)]
    pub struct HeartBeat {
        // TODO: consider using `Interval` directly instead of `IntervalStream`
        #[pin]
        interval: Option<IntervalStream>,
    }
}

impl HeartBeat {
    /// A [`HeartBeat`] that will never yield `Poll::Ready(_)` with `StreamExt::next()`
    pub fn never() -> Self {
        Self { interval: None }
    }

    /// A [`HeartBeat`] that will yield `Poll::Ready(_)` per the given interval with `StreamExt::next()`
    pub fn new(period: Duration) -> Self {
        Self {
            interval: Some(IntervalStream::new(period)),
        }
    }
}

impl Stream for HeartBeat {
    type Item = io::Result<()>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.project();
        match this.interval.as_pin_mut() {
            Some(interval) => match interval.poll_next(cx) {
                Poll::Ready(Some(_instant)) => Poll::Ready(Some(Ok(()))),
                Poll::Ready(None) => Poll::Ready(None),
                Poll::Pending => Poll::Pending,
            },
            None => Poll::Pending,
        }
    }
}
