use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Notify;

use super::Producer;

#[derive(Debug)]
pub struct Consumer<State> {
    pub notifier: Arc<Notify>,
    state: State,
}

impl<State> Consumer<State> {
    pub fn new(notifier: Arc<Notify>, state: State) -> Self {
        Self { notifier, state }
    }

    pub fn state(&self) -> &State {
        &self.state
    }
}

impl<State> AsRef<State> for Consumer<Arc<State>> {
    fn as_ref(&self) -> &State {
        &self.state
    }
}

impl<State: Clone> Consumer<State> {
    pub fn producer(&self) -> Producer<State> {
        Producer::new(self.notifier.clone(), self.state.clone())
    }
}

#[async_trait]
pub trait Consume {
    type Item: Send;
    type Outcome: Send;

    async fn consume(&mut self, item: Self::Item) -> Self::Outcome;
}
