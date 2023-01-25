#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs, missing_debug_implementations)]

//! WebSocket adapter for AMQP 1.0 websocket binding
//!
//! This provides a thin wrapper over `tokio_tungstenite::WebSocketStream`, and the wrapper performs
//! the WebSocket handshake with the "Sec-WebSocket-Protocol" HTTP header set to "amqp".
//!
//! The wrapper type [`WebSocketStream`] could also be used for non-AMQP applications; however, the
//! user should establish websocket stream with raw `tokio_tungstenite` API and then wrap the stream
//! with the wrapper by `fe2o3_amqp_ws::WebSocketStream::from(ws_stream)`.
//!
//! # Feature flags
//!
//! ```toml
//! default = []
//! ```
//!
//! | Feature | Description |
//! |---------|-------------|
//! | `native-tls` | Enables "tokio-tungstenite/native-tls" |
//! | `native-tls-vendored` | Enables "tokio-tungstenite/native-tls-vendored" |
//! | `rustls-tls-native-roots` | Enables "tokio-tungstenite/rustls-tls-native-roots" |
//! | `rustls-tls-webpki-roots` | Enables "tokio-tungstenite/rustls-tls-webpki-roots" |
//!
//! # Example
//!
//! ```rust
//! use fe2o3_amqp::{
//!     types::{messaging::Outcome, primitives::Value},
//!     Connection, Delivery, Receiver, Sender, Session,
//! };
//! use fe2o3_amqp_ws::WebSocketStream;
//!
//! #[tokio::main]
//! async fn main() {
//!     let ws_stream = WebSocketStream::connect("ws://localhost:5673")
//!         .await
//!         .unwrap();
//!     let mut connection = Connection::builder()
//!         .container_id("connection-1")
//!         .open_with_stream(ws_stream)
//!         .await
//!         .unwrap();
//!     let mut session = Session::begin(&mut connection).await.unwrap();
//!
//!     let mut sender = Sender::attach(&mut session, "rust-sender-link-1", "q1")
//!         .await
//!         .unwrap();
//!     let mut receiver = Receiver::attach(&mut session, "rust-recver-1", "q1")
//!         .await
//!         .unwrap();
//!
//!     let fut = sender.send_batchable("hello batchable AMQP").await.unwrap();
//!
//!     let delivery: Delivery<Value> = receiver.recv().await.unwrap();
//!     receiver.accept(&delivery).await.unwrap();
//!
//!     let outcome: Outcome = fut.await.unwrap();
//!     outcome.accepted_or_else(|state| state).unwrap(); // Handle delivery outcome
//!
//!     sender.close().await.unwrap();
//!     receiver.close().await.unwrap();
//!     session.end().await.unwrap();
//!     connection.close().await.unwrap();
//! }
//! ```
//! 
//! ## WebAssembly support
//! 
//! Experimental support for `wasm32-unknown-unknown` target has been added since "0.3.0" and uses a
//! `web_sys::WebSocket` internally. An example of this can be found in
//! [examples/wasm32-in-browser](https://github.com/minghuaw/fe2o3-amqp/tree/main/examples/wasm32-in-browser).

use std::{
    io::{self, Cursor, Read},
    task::Poll,
};

use futures_util::{ready, Sink, Stream};
use pin_project_lite::pin_project;
use tokio::{
    io::{AsyncRead, AsyncWrite},
};

mod error;
pub use error::Error;

#[macro_use]
mod macros;

cfg_not_wasm32! {
    pub mod native;
}

cfg_wasm32! {
    pub mod wasm;
}

const SEC_WEBSOCKET_PROTOCOL_AMQP: &str = "amqp";

/// This a wrapper around `tungstenite::Message`
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct WsMessage(pub tungstenite::Message);

pin_project! {
    /// A wrapper over [`tokio_tungstenite::WebSoccketStream`] that implements
    /// `tokio::io::AsyncRead` and `tokio::io::AsyncWrite`.
    ///
    /// The public APIs all internally call their equivalent in `tokio_tungstenite` and checks the
    /// response. The only difference is that the APIs will set "Sec-WebSocket-Protocol" HTTP header
    /// to "amqp".
    ///
    /// The "Sec-WebSocket-Protocol" HTTP header identifies the WebSocket subprotocol. For this
    /// AMQP WebSocket binding, the value MUST be set to the US-ASCII text string “amqp” which
    /// refers to the 1.0 version of the AMQP 1.0 or greater, with version negotiation as
    /// defined by AMQP 1.0.
    ///
    /// If the Client does not receive a response with HTTP status code 101 and an HTTP
    /// Sec-WebSocket-Protocol equal to the US-ASCII text string "amqp" then the Client MUST close
    /// the socket connection
    ///
    /// # Example
    ///
    /// ```rust
    /// use fe2o3_amqp::{
    ///     types::{messaging::Outcome, primitives::Value},
    ///     Connection, Delivery, Receiver, Sender, Session,
    /// };
    /// use fe2o3_amqp_ws::WebSocketStream;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let (ws_stream, _response) = WebSocketStream::connect("ws://localhost:5673")
    ///         .await
    ///         .unwrap();
    ///     let mut connection = Connection::builder()
    ///         .container_id("connection-1")
    ///         .open_with_stream(ws_stream)
    ///         .await
    ///         .unwrap();
    ///
    ///     // ...
    ///
    ///     connection.close().await.unwrap();
    /// }
    /// ```
    #[derive(Debug)]
    pub struct WebSocketStream<S> {
        #[pin]
        inner: S,
        current_binary: Option<std::io::Cursor<Vec<u8>>>,
    }
}

// Reference implementations:
//
// - `tokio-rw-stream-sink`
// - `rw-stream-sink`
// - `ws_stream_tungstenite`
impl<S> AsyncRead for WebSocketStream<S>
where
    S: Stream<Item = Result<WsMessage, tungstenite::Error>>,
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

            match msg.0 {
                tungstenite::Message::Text(_) => {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Text messsage is not supported",
                    )))
                }
                tungstenite::Message::Binary(vec) => *this.current_binary = Some(Cursor::new(vec)),

                // This is already handled by tungstenite
                tungstenite::Message::Ping(_) => {}
                tungstenite::Message::Pong(_) => {}

                // Let tungstenite perform close handshake
                tungstenite::Message::Close(_) => {}

                // Raw frame. Note, that you’re not going to get this value while reading the message.
                tungstenite::Message::Frame(_) => unreachable!(),
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
    S: Sink<WsMessage, Error = tungstenite::Error>,
{
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let mut this = self.project();
        ready!(this.inner.as_mut().poll_ready(cx)).map_err(map_tungstenite_error)?;
        let n = buf.len();
        let item = tungstenite::Message::binary(buf);
        let item = WsMessage(item);
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