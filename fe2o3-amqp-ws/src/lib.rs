use futures_util::{Sink, Stream};
use tokio::io::{AsyncRead, AsyncWrite};
use tungstenite::Message;

pub struct WebSocketStream<S> {
    inner: S,
}

impl<S> From<tokio_tungstenite::WebSocketStream<S>>
    for WebSocketStream<tokio_tungstenite::WebSocketStream<S>>
{
    fn from(inner: tokio_tungstenite::WebSocketStream<S>) -> Self {
        Self { inner }
    }
}

impl<S> AsyncRead for WebSocketStream<S>
where
    S: Sink<Message>,
{
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        todo!()
    }
}

impl<S> AsyncWrite for WebSocketStream<S>
where
    S: Stream<Item = Message>,
{
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        todo!()
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        todo!()
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
