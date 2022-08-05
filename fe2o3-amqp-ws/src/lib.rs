#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs, missing_debug_implementations)]

//! WebSocket adapter for AMQP 1.0 websocket binding
//!
//! This provides a thin wrapper over `tokio_tungstenite::WebSocketStream`, and the wrapper
//! performs the WebSocket handshake with the "Sec-WebSocket-Protocol" HTTP header set to "amqp".
//!
//! The wrapper type [`WebSocketStream`] could also be used for non-AMQP applications; however,
//! the user should establish websocket stream with raw `tokio_tungstenite` API and then
//! wrap the stream with the wrapper by `fe2o3_amqp_ws::WebSocketStream::from(ws_stream)`.
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
//!     let (ws_stream, _response) = WebSocketStream::connect("ws://localhost:5673")
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

const SEC_WEBSOCKET_PROTOCOL: &str = "Sec-WebSocket-Protocol";

use std::{
    io::{self, Cursor, Read},
    task::Poll,
};

use futures_util::{ready, Sink, Stream};
use pin_project_lite::pin_project;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use tokio_tungstenite::{
    client_async, client_async_with_config, connect_async, connect_async_with_config,
    MaybeTlsStream,
};
use tungstenite::{
    client::IntoClientRequest,
    handshake::client::{Request, Response},
    http::HeaderValue,
    protocol::WebSocketConfig,
    Message,
};

mod error;
pub use error::Error;

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

impl WebSocketStream<MaybeTlsStream<TcpStream>> {
    /// Calls [`tokio_tungstenite::connect_async`] internally with `"Sec-WebSocket-Protocol"` HTTP
    /// header of the `req` set to `"amqp"`
    pub async fn connect(req: impl IntoClientRequest) -> Result<(Self, Response), Error> {
        let request = map_amqp_websocket_request(req)?;
        let (mut ws_stream, response) = connect_async(request).await?;
        match verify_response(response) {
            Ok(response) => Ok((Self::from(ws_stream), response)),
            Err(error) => {
                ws_stream.close(None).await?;
                Err(error)
            }
        }
    }

    /// Calls [`tokio_tungstenite::connect_async_with_config`] internally with
    /// `"Sec-WebSocket-Protocol"` HTTP header of the `req` set to `"amqp"`
    pub async fn connect_with_config(
        req: impl IntoClientRequest,
        config: Option<WebSocketConfig>,
    ) -> Result<(Self, Response), Error> {
        let request = map_amqp_websocket_request(req)?;
        let (mut ws_stream, response) = connect_async_with_config(request, config).await?;
        match verify_response(response) {
            Ok(response) => Ok((Self::from(ws_stream), response)),
            Err(error) => {
                ws_stream.close(None).await?;
                Err(error)
            }
        }
    }
}

impl<S> WebSocketStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    /// Calls [`tokio_tungstenite::client_async`] internally with `"Sec-WebSocket-Protocol"` HTTP
    /// header of the `req` set to `"amqp"`
    pub async fn connect_with_stream(
        req: impl IntoClientRequest,
        stream: S,
    ) -> Result<(Self, Response), Error> {
        let request = map_amqp_websocket_request(req)?;
        let (mut ws_stream, response) = client_async(request, stream).await?;
        match verify_response(response) {
            Ok(response) => Ok((Self::from(ws_stream), response)),
            Err(error) => {
                ws_stream.close(None).await?;
                Err(error)
            }
        }
    }

    /// Calls [`tokio_tungstenite::client_async_with_config`] internally with
    /// `"Sec-WebSocket-Protocol"` HTTP header of the `req` set to `"amqp"`
    pub async fn connect_with_stream_and_config(
        req: impl IntoClientRequest,
        stream: S,
        config: Option<WebSocketConfig>,
    ) -> Result<(Self, Response), Error> {
        let request = map_amqp_websocket_request(req)?;
        let (mut ws_stream, response) = client_async_with_config(request, stream, config).await?;
        match verify_response(response) {
            Ok(response) => Ok((Self::from(ws_stream), response)),
            Err(error) => {
                ws_stream.close(None).await?;
                Err(error)
            }
        }
    }
}

#[cfg_attr(
    docsrs,
    doc(cfg(any(
        feature = "native-tls",
        feature = "native-tls-vendored",
        feature = "rustls-tls-native-roots",
        feature = "rustls-tls-webpki-roots"
    )))
)]
#[cfg(any(
    feature = "native-tls",
    feature = "native-tls-vendored",
    feature = "rustls-tls-native-roots",
    feature = "rustls-tls-webpki-roots"
))]
impl<S> WebSocketStream<MaybeTlsStream<S>>
where
    S: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static,
{
    /// Calls [`tokio_tungstenite::client_async_tls`] internally with `"Sec-WebSocket-Protocol"` HTTP
    /// header of the `req` set to `"amqp"`
    pub async fn connect_tls_with_stream(
        req: impl IntoClientRequest,
        stream: S,
    ) -> Result<(Self, Response), Error> {
        let request = map_amqp_websocket_request(req)?;
        let (mut ws_stream, response) =
            tokio_tungstenite::client_async_tls(request, stream).await?;
        match verify_response(response) {
            Ok(response) => Ok((Self::from(ws_stream), response)),
            Err(error) => {
                ws_stream.close(None).await?;
                Err(error)
            }
        }
    }

    /// Calls [`tokio_tungstenite::client_async_tls_with_config`] internally with
    /// `"Sec-WebSocket-Protocol"` HTTP header of the `req` set to `"amqp"`
    pub async fn connect_tls_with_stream_and_config(
        req: impl IntoClientRequest,
        stream: S,
        config: Option<WebSocketConfig>,
        connector: Option<tokio_tungstenite::Connector>,
    ) -> Result<(Self, Response), Error> {
        let request = map_amqp_websocket_request(req)?;
        let (mut ws_stream, response) =
            tokio_tungstenite::client_async_tls_with_config(request, stream, config, connector)
                .await?;
        match verify_response(response) {
            Ok(response) => Ok((Self::from(ws_stream), response)),
            Err(error) => {
                ws_stream.close(None).await?;
                Err(error)
            }
        }
    }
}

#[cfg_attr(
    docsrs,
    doc(cfg(any(
        feature = "native-tls",
        feature = "native-tls-vendored",
        feature = "rustls-tls-native-roots",
        feature = "rustls-tls-webpki-roots"
    )))
)]
#[cfg(any(
    feature = "native-tls",
    feature = "native-tls-vendored",
    feature = "rustls-tls-native-roots",
    feature = "rustls-tls-webpki-roots"
))]
impl WebSocketStream<MaybeTlsStream<TcpStream>> {
    /// Calls [`tokio_tungstenite::connect_async_tls_with_config`] internally with
    /// `"Sec-WebSocket-Protocol"` HTTP header of the `req` set to `"amqp"`
    pub async fn connect_tls_with_config(
        req: impl IntoClientRequest,
        config: Option<WebSocketConfig>,
        connector: Option<tokio_tungstenite::Connector>,
    ) -> Result<(Self, Response), Error> {
        let request = map_amqp_websocket_request(req)?;
        let (mut ws_stream, response) =
            tokio_tungstenite::connect_async_tls_with_config(request, config, connector).await?;
        match verify_response(response) {
            Ok(response) => Ok((Self::from(ws_stream), response)),
            Err(error) => {
                ws_stream.close(None).await?;
                Err(error)
            }
        }
    }
}

// Reference implementations:
//
// - `tokio-rw-stream-sink`
// - `rw-stream-sink`
// - `ws_stream_tungstenite`
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
                Message::Ping(_) => {}
                Message::Pong(_) => {}

                // Let tungstenite perform close handshake
                Message::Close(_) => {}

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
        ready!(this.inner.as_mut().poll_ready(cx)).map_err(map_tungstenite_error)?;
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

fn map_amqp_websocket_request(req: impl IntoClientRequest) -> Result<Request, tungstenite::Error> {
    let mut request = req.into_client_request()?;

    // Sec-WebSocket-Protocol HTTP header
    //
    // Identifies the WebSocket subprotocol. For this AMQP WebSocket binding, the value MUST be
    // set to the US- ASCII text string “amqp” which refers to the 1.0 version of the AMQP 1.0
    // or greater, with version negotiation as defined by AMQP 1.0.
    request
        .headers_mut()
        .insert(SEC_WEBSOCKET_PROTOCOL, HeaderValue::from_static("amqp"));

    Ok(request)
}

fn verify_response(response: Response) -> Result<Response, Error> {
    use http::StatusCode;

    // If the Client does not receive a response with HTTP status code 101 and an HTTP
    // Sec-WebSocket-Protocol equal to the US-ASCII text string “amqp” then the Client MUST close
    // the socket connection
    if response.status() != StatusCode::SWITCHING_PROTOCOLS {
        return Err(Error::StatucCodeIsNotSwitchingProtocols);
    }

    match response
        .headers()
        .get(SEC_WEBSOCKET_PROTOCOL)
        .map(|val| val.to_str())
        .ok_or(Error::MissingSecWebSocketProtocol)??
    {
        "amqp" => Ok(response),
        _ => Err(Error::SecWebSocketProtocolIsNotAmqp),
    }
}
