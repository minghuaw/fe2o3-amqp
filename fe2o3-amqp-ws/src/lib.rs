#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs, missing_debug_implementations)]

//! WebSocket adapter for AMQP 1.0 websocket binding
//!
//! This provides a thin wrapper over `tokio_tungstenite::WebSocketStream`, and the wrapper performs
//! the WebSocket handshake with the "Sec-WebSocket-Protocol" HTTP header set to "amqp".
//!
//! Every public constructor of [`WebSocketStream`] sets the "Sec-WebSocket-Protocol" header to
//! "amqp" and rejects a handshake response that does not return the same value. To do the
//! handshake over a stream that you open yourself, use `WebSocketStream::connect_with_stream` or
//! `WebSocketStream::connect_with_stream_and_config`. This crate has no public constructor that
//! wraps a `tokio_tungstenite::WebSocketStream` that you built with the raw `tokio_tungstenite`
//! API.
//!
//! # Re-exports
//!
//! This crate re-exports `tungstenite` and `tokio_tungstenite` so that downstream
//! users can reference the same versions used internally:
//!
//! ```rust
//! use fe2o3_amqp_ws::tungstenite;
//! use fe2o3_amqp_ws::tokio_tungstenite;
//! ```
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
//! # TLS
//!
//! These three connect functions are not feature gated:
//! `WebSocketStream::connect_tls_with_config`, `WebSocketStream::connect_tls_with_stream_and_config`,
//! and `WebSocketStream::connect_tls_with_stream`. The first two take a TLS `connector`. The third
//! is a shorthand that supplies no connector. A library crate can depend on `fe2o3-amqp-ws` with
//! `default-features = false`, call these functions, and let the application that builds the final
//! binary select the TLS stack.
//!
//! The four features above select which TLS stack `tokio-tungstenite` links. They do not control
//! whether the connect functions exist. Enable the feature on `fe2o3-amqp-ws` itself. A TLS
//! feature that another crate enables on `tokio-tungstenite` does not enable the TLS code path of
//! this crate, because a crate cannot read the features of its dependency.
//!
//! With none of the four features enabled, these three functions have no TLS stack. A `ws://`
//! address connects in plaintext. A `wss://` address returns
//! `Error::Tungstenite(tungstenite::Error::Url(tungstenite::error::UrlError::TlsFeatureNotEnabled))`
//! before a socket is opened, for every value of `connector`. A `wss://` address is never
//! downgraded to a plaintext connection.
//!
//! `WebSocketStream::connect` and `WebSocketStream::connect_with_config` behave differently. They
//! take no connector and pass `None` to `tokio-tungstenite`, which then selects the TLS stack from
//! its own enabled features. Cargo unifies features over the whole dependency graph, so another
//! crate can change that selection. These two functions can therefore make a TLS connection when
//! `fe2o3-amqp-ws` has no TLS feature enabled. Call `WebSocketStream::connect_tls_with_config` with
//! `Some(connector)` for a deterministic TLS stack.
//!
//! # Example
//!
//! ```rust,no_run
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

use bytes::Bytes;
use futures_util::{ready, Sink, Stream};
use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, AsyncWrite};

mod error;
pub use error::Error;

pub use tungstenite;

#[cfg(not(target_arch = "wasm32"))]
pub use tokio_tungstenite;

#[macro_use]
mod macros;

cfg_not_wasm32! {
    pub mod native;
}

cfg_wasm32! {
    pub mod wasm;
}

const SEC_WEBSOCKET_PROTOCOL_AMQP: &str = "amqp";

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
    /// ```rust,no_run
    /// use fe2o3_amqp::{
    ///     types::{messaging::Outcome, primitives::Value},
    ///     Connection, Delivery, Receiver, Sender, Session,
    /// };
    /// use fe2o3_amqp_ws::WebSocketStream;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let ws_stream = WebSocketStream::connect("ws://localhost:5673")
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
        current_binary: Option<std::io::Cursor<Bytes>>,
    }
}

// Reference implementations:
//
// - `tokio-rw-stream-sink`
// - `rw-stream-sink`
// - `ws_stream_tungstenite`
impl<S> AsyncRead for WebSocketStream<S>
where
    S: Stream<Item = Result<tungstenite::Message, Error>>,
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
                Some(Err(err)) => return Poll::Ready(Err(map_stream_error(err))),
                None => return Poll::Ready(Ok(())), // EOF
            };

            match msg {
                tungstenite::Message::Text(_) => {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Text messsage is not supported",
                    )))
                }
                tungstenite::Message::Binary(buf) => *this.current_binary = Some(Cursor::new(buf)),

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
    S: Sink<tungstenite::Message, Error = Error>,
{
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let mut this = self.project();
        ready!(this.inner.as_mut().poll_ready(cx)).map_err(map_stream_error)?;
        let n = buf.len();
        let bin = Bytes::copy_from_slice(buf);
        let item = tungstenite::Message::binary(bin);
        match this.inner.start_send(item) {
            Ok(_) => Poll::Ready(Ok(n)),
            Err(error) => Poll::Ready(Err(map_stream_error(error))),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        let this = self.project();
        this.inner.poll_flush(cx).map_err(map_stream_error)
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        let this = self.project();
        this.inner.poll_close(cx).map_err(map_stream_error)
    }
}

fn map_stream_error(error: Error) -> io::Error {
    match error {
        Error::Tungstenite(e) => match e {
            tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed => {
                io::ErrorKind::NotConnected.into()
            }
            tungstenite::Error::Io(err) => err,
            tungstenite::Error::Capacity(err) => io::Error::new(io::ErrorKind::InvalidData, err),
            other => io::Error::other(other),
        },
        other => io::Error::other(other),
    }
}
