use std::{
    io::{self, Cursor, Read},
    task::Poll,
};

use futures_util::{ready, Sink, Stream};
use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, AsyncWrite};
use tungstenite::Message;

pin_project! {
    pub struct WebSocketStream<S> {
        #[pin]
        inner: tokio_tungstenite::WebSocketStream<S>,
        current_binary: Option<std::io::Cursor<Vec<u8>>>,
    }
}

impl<S> From<tokio_tungstenite::WebSocketStream<S>> for WebSocketStream<S> {
    fn from(inner: tokio_tungstenite::WebSocketStream<S>) -> Self {
        Self {
            inner,
            current_binary: None,
        }
    }
}

// Reference implementations:
//
// - `tokio-rw-stream-sink`
impl<S> AsyncRead for WebSocketStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.project();
        let mut inner = this.inner;

        let (item_to_copy, len_to_read) = loop {
            if let Some(cursor) = this.current_binary {
                let len = cursor.get_ref().len() as u64;
                let pos = cursor.position();
                if pos < len {
                    break (cursor, len - pos);
                }
            }

            let msg = match ready!(inner.as_mut().poll_next(cx)) {
                Some(Ok(msg)) => msg,
                Some(Err(err)) => return Poll::Ready(Err(map_tungstenite_error(err))),
                None => return Poll::Ready(Ok(())), // EOF
            };

            match msg {
                Message::Text(_) => {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Text messsage is not supported",
                    )))
                }
                Message::Binary(vec) => *this.current_binary = Some(Cursor::new(vec)),

                // This is already handled by tungstenite
                Message::Ping(_) => {},
                Message::Pong(_) => {},

                // Let tungstenite perform close handshake
                Message::Close(_) => {},

                // Raw frame. Note, that you’re not going to get this value while reading the message.
                Message::Frame(_) => unreachable!(), 
            }
        };

        // Copy it!
        let len_to_read = buf
            .remaining()
            .min(len_to_read.min(usize::MAX as u64) as usize);
        let unfilled_buf = buf.initialize_unfilled_to(len_to_read);
        let len = item_to_copy.read(unfilled_buf)?;
        buf.advance(len);
        Poll::Ready(Ok(()))
    }
}

impl<S> AsyncWrite for WebSocketStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let mut this = self.project();
        ready!(this.inner.as_mut().poll_ready(cx))
            .map_err(map_tungstenite_error)?;
        let n = buf.len();
        let item = Message::binary(buf);
        match this.inner.start_send(item) {
            Ok(_) => Poll::Ready(Ok(n)),
            Err(error) => Poll::Ready(Err(map_tungstenite_error(error))),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        let this = self.project();
        this.inner.poll_flush(cx).map_err(map_tungstenite_error)
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        let this = self.project();
        this.inner.poll_close(cx).map_err(map_tungstenite_error)
    }
}

fn map_tungstenite_error(error: tungstenite::Error) -> io::Error {
    match error {
        tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed => {
            io::ErrorKind::NotConnected.into()
        }
        tungstenite::Error::Io(err) => err,
        tungstenite::Error::Capacity(err) => io::Error::new(io::ErrorKind::InvalidData, err),
        _ => io::Error::new(io::ErrorKind::Other, error),
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
