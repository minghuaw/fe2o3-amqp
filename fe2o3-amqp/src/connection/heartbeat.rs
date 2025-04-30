//! Implements an asynchronous heartbeat

use std::{io, task::{Context, Poll}, time::Duration};

use futures_util::Stream;
use pin_project_lite::pin_project;

/// A helper trait to abstract over the `Interval` type used in the heartbeat implementation.
pub trait HeartBeatInterval: Unpin {
    /// Type of the instant returned by the interval.
    type Instant;

    /// Create a new interval with the given period.
    fn new_with_period(period: Duration) -> Self;

    /// Poll the interval for a tick.
    fn poll_tick(&mut self, cx: &mut Context<'_>) -> Poll<Self::Instant>;
}

cfg_not_wasm32! {
    type Interval = tokio::time::Interval;

    impl HeartBeatInterval for tokio::time::Interval {
        type Instant = tokio::time::Instant;

        fn new_with_period(period: Duration) -> Self {
            tokio::time::interval(period)
        }
    
        fn poll_tick(&mut self, cx: &mut Context<'_>) -> Poll<Self::Instant> {
            self.poll_tick(cx)
        }
    }
}

cfg_wasm32! {
    type Interval = wasmtimer::tokio::Interval;

    impl HeartBeatInterval for wasmtimer::tokio::Interval {
        type Instant = wasmtimer::std::Instant;

        fn new_with_period(period: Duration) -> Self {
            wasmtimer::tokio::interval(period)
        }
    
        fn poll_tick(&mut self, cx: &mut Context<'_>) -> Poll<Self::Instant> {
            self.poll_tick(cx)
        }
    }
}

pin_project! {
    /// A wrapper over an `Option<IntervalStream>` which will never tick ready if the underlying
    /// `Interval` is `None`
    #[derive(Debug)]
    pub struct HeartBeat<T = Interval> 
    where
        T: HeartBeatInterval,
    {
        #[pin]
        interval: Option<T>,
    }
}

impl<T> HeartBeat<T> 
where 
    T: HeartBeatInterval,
{
    /// A [`HeartBeat`] that will never yield `Poll::Ready(_)` with `StreamExt::next()`
    pub fn never() -> Self {
        Self { interval: None }
    }

    /// A [`HeartBeat`] that will yield `Poll::Ready(_)` per the given interval with `StreamExt::next()`
    pub fn new(period: Duration) -> Self {
        let interval = Some(T::new_with_period(period));
        Self { interval }
    }
}

impl<T> Stream for HeartBeat<T> 
where 
    T: HeartBeatInterval,
{
    type Item = io::Result<()>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.project();
        let interval = this.interval.get_mut();
        match interval {
            Some(interval) => interval.poll_tick(cx).map(|_| Some(Ok(()))),
            None => Poll::Pending,
        }
    }
}
