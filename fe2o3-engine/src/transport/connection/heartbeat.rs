use std::{task::Poll, time::Duration};

use futures_util::{Stream};
use pin_project_lite::pin_project;

use tokio::{time::{Instant}};
use tokio_stream::wrappers::IntervalStream;

pin_project! {
    /// A wrapper over an `Option<IntervalStream>` which will never tick ready if the underlying 
    /// `Interval` is `None`
    pub struct HeartBeat {
        #[pin]
        interval: Option<IntervalStream>
    }
}

impl HeartBeat {
    pub fn never() -> Self {
        Self {
            interval: None
        }
    }

    pub fn new(period: Duration) -> Self {
        let interval = Some(
            IntervalStream::new(tokio::time::interval(period))
        );
        Self { interval }
    }

    // pub fn set_period(&mut self, period: Duration) -> &mut Self {
    //     let interval = Some(
    //         IntervalStream::new(tokio::time::interval(period))
    //     );
    //     self.interval = interval;
    //     self
    // }
}

impl Stream for HeartBeat {
    type Item = Instant;

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        let this = self.project();
        match this.interval.as_pin_mut() {
            Some(stream) => {
                stream.poll_next(cx)
            },
            None => Poll::Pending
        }
    }   
}