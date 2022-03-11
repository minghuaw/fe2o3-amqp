use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Notify;

#[derive(Debug)]
pub struct Producer<State> {
    pub notifier: Arc<Notify>,
    state: State,
}

impl<State> Producer<State> {
    pub fn new(notifier: Arc<Notify>, state: State) -> Self {
        Self { notifier, state }
    }
}

#[async_trait]
pub trait Produce {
    type Item: Send;
    type Outcome: Send;

    async fn produce(&mut self, item: Self::Item) -> Self::Outcome;
}

#[async_trait]
pub trait ProducerState {
    type Item: Send;
    type Outcome: Send;

    async fn update_state(&mut self, item: Self::Item) -> Self::Outcome;
}

#[async_trait]
impl<T> Produce for Producer<T>
where
    T: ProducerState + Send,
{
    type Item = T::Item;
    type Outcome = T::Outcome;

    async fn produce(&mut self, item: Self::Item) -> Self::Outcome {
        let outcome = self.state.update_state(item).await;
        self.notifier.notify_waiters();
        outcome
    }
}
