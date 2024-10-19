use std::future::Future;

pub trait Runtime {
    type JoinHandle<T>: Future<Output = T> + Send + 'static;

    fn spawn<F>(&self, f: F) -> Self::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static;
}