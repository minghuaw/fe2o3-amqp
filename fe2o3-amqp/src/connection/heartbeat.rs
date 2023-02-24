//! Implements an asynchronous heartbeat

use std::{io, pin::Pin, task::Poll, time::Duration};

use futures_util::Stream;
use pin_project_lite::pin_project;

cfg_not_wasm32! {
    use tokio_stream::wrappers::IntervalStream;

    #[derive(Debug)]
    struct InnerStream {
        interval: IntervalStream,
    }

    impl InnerStream {
        fn new(period: Duration) -> Self {
            let interval = tokio::time::interval(period);
            let interval = IntervalStream::new(interval);
            Self { interval }
        }
    }

    impl Stream for InnerStream {
        type Item = io::Result<()>;

        fn poll_next(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Self::Item>> {
            let interval = Pin::new(&mut self.interval);
            match interval.poll_next(cx) {
                Poll::Ready(Some(_instant)) => Poll::Ready(Some(Ok(()))),
                Poll::Ready(None) => Poll::Ready(None),
                Poll::Pending => Poll::Pending,
            }
        }
    }
}

cfg_wasm32! {
    use fluvio_wasm_timer::{Delay};
    use futures_util::{Future, ready};

    #[derive(Debug)]
    struct InnerStream {
        delay: Delay,
        period: Duration,
    }

    impl InnerStream {
        fn new(period: Duration) -> Self {
            let delay = Delay::new(period);
            Self { delay, period }
        }
    }

    impl Stream for InnerStream {
        type Item = io::Result<()>;

        fn poll_next(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Self::Item>> {
            ready!(Pin::new(&mut self.delay).poll(cx))?;
            let period = self.period;
            self.delay.reset(period);
            Poll::Ready(Some(Ok(())))
        }
    }
}

pin_project! {
    /// A wrapper over an `Option<IntervalStream>` which will never tick ready if the underlying
    /// `Interval` is `None`
    #[derive(Debug)]
    pub struct HeartBeat {
        #[pin]
        interval: Option<InnerStream>,
    }
}

impl HeartBeat {
    /// A [`HeartBeat`] that will never yield `Poll::Ready(_)` with `StreamExt::next()`
    pub fn never() -> Self {
        Self { interval: None }
    }

    /// A [`HeartBeat`] that will yield `Poll::Ready(_)` per the given interval with `StreamExt::next()`
    pub fn new(period: Duration) -> Self {
        let interval = Some(InnerStream::new(period));
        Self { interval }
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
            Some(stream) => stream.poll_next(cx),
            None => Poll::Pending,
        }
    }
}
