// pub enum Running {
//     Stop,
//     Continue,
// }

use std::{pin::Pin, task::Poll, time::{Duration}};
use futures_util::Future;
use tokio::time::{Instant};

use tokio::time::Sleep;

// pub struct IdleTimeoutElapsed {}

pub struct IdleTimeout {
    delay: Pin<Box<Sleep>>,
    duration: Duration
}


impl IdleTimeout {
    pub fn new(duration: Duration) -> Self {
        let delay = Box::pin(tokio::time::sleep(duration));
        Self { delay, duration }
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }

    pub fn reset(&mut self) {
        let now = Instant::now();
        let next= now + self.duration;
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

