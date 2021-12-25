use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Notify;

pub struct Consumer<State> {
    pub notifier: Arc<Notify>,
    state: State,
}

impl<State> Consumer<State> {
    pub fn new(notifier: Arc<Notify>, state: State) -> Self {
        Self {
            notifier,
            state
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }
}

#[async_trait]
pub trait Consume {
    type Item: Send;
    type Outcome: Send;

    async fn consume(&mut self, item: Self::Item) -> Self::Outcome;
}