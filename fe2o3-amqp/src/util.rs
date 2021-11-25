// pub enum Running {
//     Stop,
//     Continue,
// }

use futures_util::Future;
use std::ops::Deref;
use std::{pin::Pin, task::Poll, time::Duration};
use tokio::time::Instant;

use tokio::time::Sleep;

// pub struct IdleTimeoutElapsed {}

pub enum Running {
    Continue,
    Stop,
}

pub struct IdleTimeout {
    delay: Pin<Box<Sleep>>,
    duration: Duration,
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
pub struct Constant<T> {
    value: T
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