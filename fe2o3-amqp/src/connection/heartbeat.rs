//! Implements an asynchronous heartbeat

use std::{task::Poll, time::Duration};

use futures_util::Stream;
use pin_project_lite::pin_project;

use tokio::time::Instant;
use tokio_stream::wrappers::IntervalStream;

pin_project! {
    /// A wrapper over an `Option<IntervalStream>` which will never tick ready if the underlying
    /// `Interval` is `None`
    #[derive(Debug)]
    pub struct HeartBeat {
        #[pin]
        interval: Option<IntervalStream>
    }
}

impl HeartBeat {
    /// A [`HeartBeat`] that will never yield `Poll::Ready(_)` with `StreamExt::next()`
    pub fn never() -> Self {
        Self { interval: None }
    }

    /// A [`HeartBeat`] that will yield `Poll::Ready(_)` per the given interval with `StreamExt::next()`
    pub fn new(period: Duration) -> Self {
        let interval = Some(IntervalStream::new(tokio::time::interval(period)));
        Self { interval }
    }
}

impl Stream for HeartBeat {
    type Item = Instant;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.project();
        match this.interval.as_pin_mut() {
            Some(stream) => stream.poll_next(cx),
            None => Poll::Pending,
        }
    }
}
