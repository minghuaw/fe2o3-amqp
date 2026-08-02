//! WebSocket wrapper over native tokio-tungstenite WebSocketStream

use futures_util::{Stream, Sink};
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
};

use super::{Error, WebSocketStream};

const SEC_WEBSOCKET_PROTOCOL: &str = "Sec-WebSocket-Protocol";

pin_project! {
    /// This a simple wrapper around [`tokio_tungstenite::WebSocketStream`]
    #[derive(Debug)]
    pub struct TokioWebSocketStream<S>{
        #[pin]
        stream: tokio_tungstenite::WebSocketStream<S>,
        response: Response,
    }
}

impl<S> From<TokioWebSocketStream<S>> for WebSocketStream<TokioWebSocketStream<S>> {
    fn from(inner: TokioWebSocketStream<S>) -> Self {
        Self {
            inner,
            current_binary: None,
        }
    }
}

impl<S> TokioWebSocketStream<S> {
    fn new(stream: tokio_tungstenite::WebSocketStream<S>, response: Response) -> Self {
        Self { stream, response }
    }
}

impl<S> Stream for TokioWebSocketStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    type Item = Result<tungstenite::Message, Error>;

    #[allow(
        clippy::result_large_err,
        reason = "boxing would change the public trait associated type"
    )]
    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.project();
        this.stream
            .poll_next(cx)
            .map(|item| item.map(|item| item.map_err(Error::from)))
    }
}

impl<S> Sink<tungstenite::Message> for TokioWebSocketStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    type Error = Error;

    fn poll_ready(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.stream.poll_ready(cx).map_err(Error::from)
    }

    fn start_send(self: std::pin::Pin<&mut Self>, item: tungstenite::Message) -> Result<(), Self::Error> {
        let this = self.project();
        this.stream.start_send(item).map_err(Error::from)
    }

    fn poll_flush(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.stream.poll_flush(cx).map_err(Error::from)
    }

    fn poll_close(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.stream.poll_close(cx).map_err(Error::from)
    }
}

impl WebSocketStream<TokioWebSocketStream<MaybeTlsStream<TcpStream>>> {
    /// Calls [`tokio_tungstenite::connect_async`] internally with `"Sec-WebSocket-Protocol"` HTTP
    /// header of the `req` set to `"amqp"`
    ///
    /// This function takes no TLS connector. For a `wss://` address, `tokio_tungstenite` selects
    /// the TLS stack from its own enabled features, and it prefers `native-tls` over `rustls`.
    /// Cargo unifies features over the whole dependency graph, so another crate can change that
    /// selection. Call [`Self::connect_tls_with_config`] with `Some(connector)` to select the TLS
    /// stack yourself.
    pub async fn connect(addr: impl AsRef<str>) -> Result<Self, Error> {
        let req = addr.as_ref();
        let request = map_amqp_websocket_request(req)?;
        let (mut ws_stream, response) = connect_async(request).await?;
        match verify_response(response) {
            Ok(response) => Ok(Self::from(TokioWebSocketStream::new(ws_stream, response))),
            Err(error) => {
                ws_stream.close(None).await?;
                Err(error)
            }
        }
    }

    /// Calls [`tokio_tungstenite::connect_async_with_config`] internally with
    /// `"Sec-WebSocket-Protocol"` HTTP header of the `req` set to `"amqp"`
    /// 
    /// `disable_nagle` specifies if the Nagle’s algorithm must be disabled,
    /// i.e. `set_nodelay(true)`. If you don’t know what the Nagle’s algorithm is,
    /// better leave it set to `false`.
    ///
    /// This function takes no TLS connector. For a `wss://` address, `tokio_tungstenite` selects
    /// the TLS stack from its own enabled features, and it prefers `native-tls` over `rustls`.
    /// Cargo unifies features over the whole dependency graph, so another crate can change that
    /// selection. Call [`Self::connect_tls_with_config`] with `Some(connector)` to select the TLS
    /// stack yourself.
    pub async fn connect_with_config(
        addr: impl AsRef<str>,
        config: Option<WebSocketConfig>,
        disable_nagle: bool,
    ) -> Result<Self, Error> {
        let req = addr.as_ref();
        let request = map_amqp_websocket_request(req)?;
        let (mut ws_stream, response) = connect_async_with_config(request, config, disable_nagle).await?;
        match verify_response(response) {
            Ok(response) => Ok(Self::from(TokioWebSocketStream::new(ws_stream, response))),
            Err(error) => {
                ws_stream.close(None).await?;
                Err(error)
            }
        }
    }

    /// Connects to `addr` with an explicit TLS `connector`, with the `"Sec-WebSocket-Protocol"`
    /// HTTP header of the request set to `"amqp"`.
    ///
    /// This function is not feature gated. A library crate can call it with no TLS feature of this
    /// crate enabled, and the application that builds the final binary selects the TLS stack. See
    /// issue [#356](https://github.com/minghuaw/fe2o3-amqp/issues/356).
    ///
    /// `disable_nagle` specifies if the Nagle's algorithm must be disabled, i.e.
    /// `set_nodelay(true)`. If you do not know what the Nagle's algorithm is, leave it set to
    /// `false`.
    ///
    /// # With a TLS feature of this crate enabled
    ///
    /// This calls `tokio_tungstenite::connect_async_tls_with_config` internally.
    /// `Some(connector)` uses that connector. `None` lets `tokio_tungstenite` select the TLS stack
    /// from its own enabled features, and Cargo feature unification can change that selection.
    /// Pass `Some(connector)` for a deterministic TLS stack.
    ///
    /// # With no TLS feature of this crate enabled
    ///
    /// This crate has no TLS stack, so it cannot make a TLS connection.
    ///
    /// - A `ws://` address connects in plaintext. The `connector` is unused, which is also what
    ///   `tokio_tungstenite` does for a `ws://` address.
    /// - A `wss://` address returns
    ///   `Error::Tungstenite(tungstenite::Error::Url(tungstenite::error::UrlError::TlsFeatureNotEnabled))`
    ///   for every value of `connector`, and no socket is opened. A `wss://` address is never
    ///   downgraded to a plaintext connection.
    ///
    /// Enable one of `native-tls`, `native-tls-vendored`, `rustls-tls-native-roots`, or
    /// `rustls-tls-webpki-roots` on `fe2o3-amqp-ws` to make a TLS connection. A TLS feature
    /// enabled on `tokio-tungstenite` alone is not sufficient, because a crate cannot read the
    /// features of its dependency.
    pub async fn connect_tls_with_config(
        addr: impl AsRef<str>,
        config: Option<WebSocketConfig>,
        disable_nagle: bool,
        connector: Option<tokio_tungstenite::Connector>,
    ) -> Result<Self, Error> {
        let req = addr.as_ref();
        let request = map_amqp_websocket_request(req)?;
        let (mut ws_stream, response) =
            connect_async_with_connector(request, config, disable_nagle, connector).await?;
        match verify_response(response) {
            Ok(response) => Ok(Self::from(TokioWebSocketStream::new(ws_stream, response))),
            Err(error) => {
                ws_stream.close(None).await?;
                Err(error)
            }
        }
    }
}

impl<S> WebSocketStream<TokioWebSocketStream<S>>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    /// Returns the [`Response`] of the WebSocket handshake
    pub fn response(&self) -> &Response {
        &self.inner.response
    }

    /// Calls [`tokio_tungstenite::client_async`] internally with `"Sec-WebSocket-Protocol"` HTTP
    /// header of the `req` set to `"amqp"`
    pub async fn connect_with_stream(
        addr: impl AsRef<str>,
        stream: S,
    ) -> Result<Self, Error> {
        let req = addr.as_ref();
        let request = map_amqp_websocket_request(req)?;
        let (mut ws_stream, response) = client_async(request, stream).await?;
        match verify_response(response) {
            Ok(response) => Ok(Self::from(TokioWebSocketStream::new(ws_stream, response))),
            Err(error) => {
                ws_stream.close(None).await?;
                Err(error)
            }
        }
    }

    /// Calls [`tokio_tungstenite::client_async_with_config`] internally with
    /// `"Sec-WebSocket-Protocol"` HTTP header of the `req` set to `"amqp"`
    pub async fn connect_with_stream_and_config(
        addr: impl AsRef<str>,
        stream: S,
        config: Option<WebSocketConfig>,
    ) -> Result<Self, Error> {
        let req = addr.as_ref();
        let request = map_amqp_websocket_request(req)?;
        let (mut ws_stream, response) = client_async_with_config(request, stream, config).await?;
        match verify_response(response) {
            Ok(response) => Ok(Self::from(TokioWebSocketStream::new(ws_stream, response))),
            Err(error) => {
                ws_stream.close(None).await?;
                Err(error)
            }
        }
    }
}

impl<S> WebSocketStream<TokioWebSocketStream<MaybeTlsStream<S>>>
where
    S: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    /// Performs the WebSocket handshake over `stream`, upgrading it to TLS if `addr` is a `wss://`
    /// address, with the `"Sec-WebSocket-Protocol"` HTTP header of the request set to `"amqp"`.
    ///
    /// This is the same as `connect_tls_with_stream_and_config(addr, stream, None, None)`. It
    /// takes no connector, so `tokio_tungstenite` selects the TLS stack from its own enabled
    /// features. A library crate should prefer [`Self::connect_tls_with_stream_and_config`] and
    /// let the application supply the connector.
    ///
    /// This function is not feature gated. With no TLS feature of this crate enabled, a `wss://`
    /// address returns
    /// `Error::Tungstenite(tungstenite::Error::Url(tungstenite::error::UrlError::TlsFeatureNotEnabled))`
    /// and drops `stream`. See [`Self::connect_tls_with_stream_and_config`].
    pub async fn connect_tls_with_stream(addr: impl AsRef<str>, stream: S) -> Result<Self, Error> {
        Self::connect_tls_with_stream_and_config(addr, stream, None, None).await
    }

    /// Performs the WebSocket handshake over `stream` with an explicit TLS `connector`, with the
    /// `"Sec-WebSocket-Protocol"` HTTP header of the request set to `"amqp"`.
    ///
    /// This function is not feature gated. A library crate can call it with no TLS feature of this
    /// crate enabled, and the application that builds the final binary selects the TLS stack. See
    /// issue [#356](https://github.com/minghuaw/fe2o3-amqp/issues/356).
    ///
    /// # With a TLS feature of this crate enabled
    ///
    /// This calls `tokio_tungstenite::client_async_tls_with_config` internally.
    /// `Some(connector)` uses that connector. `None` lets `tokio_tungstenite` select the TLS stack
    /// from its own enabled features, and Cargo feature unification can change that selection.
    /// Pass `Some(connector)` for a deterministic TLS stack.
    ///
    /// # With no TLS feature of this crate enabled
    ///
    /// This crate has no TLS stack, so it cannot make a TLS connection.
    ///
    /// - A `ws://` address wraps `stream` in `tokio_tungstenite::MaybeTlsStream::Plain` and
    ///   completes the handshake in plaintext. The `connector` is unused, which is also what
    ///   `tokio_tungstenite` does for a `ws://` address.
    /// - A `wss://` address returns
    ///   `Error::Tungstenite(tungstenite::Error::Url(tungstenite::error::UrlError::TlsFeatureNotEnabled))`
    ///   for every value of `connector`, and no byte goes to `stream`. This function owns `stream`
    ///   and drops it, which closes it. A `wss://` address is never downgraded to a plaintext
    ///   handshake.
    ///
    /// Enable one of `native-tls`, `native-tls-vendored`, `rustls-tls-native-roots`, or
    /// `rustls-tls-webpki-roots` on `fe2o3-amqp-ws` to make a TLS connection. A TLS feature
    /// enabled on `tokio-tungstenite` alone is not sufficient, because a crate cannot read the
    /// features of its dependency.
    pub async fn connect_tls_with_stream_and_config(
        addr: impl AsRef<str>,
        stream: S,
        config: Option<WebSocketConfig>,
        connector: Option<tokio_tungstenite::Connector>,
    ) -> Result<Self, Error> {
        let req = addr.as_ref();
        let request = map_amqp_websocket_request(req)?;
        let (mut ws_stream, response) =
            client_async_with_connector(request, stream, config, connector).await?;
        match verify_response(response) {
            Ok(response) => Ok(Self::from(TokioWebSocketStream::new(ws_stream, response))),
            Err(error) => {
                ws_stream.close(None).await?;
                Err(error)
            }
        }
    }
}

// A TLS feature of this crate is enabled, so it is forwarded to `tokio-tungstenite` and the TLS
// entry points are exported. Delegate to them unchanged.
#[cfg(any(
    feature = "native-tls",
    feature = "native-tls-vendored",
    feature = "rustls-tls-native-roots",
    feature = "rustls-tls-webpki-roots"
))]
#[allow(
    clippy::result_large_err,
    reason = "large error is inherent to tungstenite"
)]
async fn connect_async_with_connector(
    request: Request,
    config: Option<WebSocketConfig>,
    disable_nagle: bool,
    connector: Option<tokio_tungstenite::Connector>,
) -> Result<
    (
        tokio_tungstenite::WebSocketStream<MaybeTlsStream<TcpStream>>,
        Response,
    ),
    tungstenite::Error,
> {
    tokio_tungstenite::connect_async_tls_with_config(request, config, disable_nagle, connector)
        .await
}

// No TLS feature of this crate is enabled, so `tokio_tungstenite` does not export
// `connect_async_tls_with_config` and this crate has no TLS stack.
//
// `tokio_tungstenite` takes the mode from the URI scheme, never from the connector. Every one of
// its `wrap_stream` implementations returns `MaybeTlsStream::Plain` for `Mode::Plain` and ignores
// the connector. This function therefore forwards a `ws://` address unchanged.
// `connect_async_with_config` passes `connector: None` down, but `Mode::Plain` never starts a TLS
// handshake, so that is safe.
//
// A `wss://` address is different. This crate cannot serve it, and the connector cannot change
// that, so this function rejects the address before it opens a socket. If it forwarded the
// address instead, a `tokio-tungstenite` TLS feature that another crate enabled would complete the
// handshake with a stack this crate never asked for.
#[cfg(not(any(
    feature = "native-tls",
    feature = "native-tls-vendored",
    feature = "rustls-tls-native-roots",
    feature = "rustls-tls-webpki-roots"
)))]
#[allow(
    clippy::result_large_err,
    reason = "large error is inherent to tungstenite"
)]
async fn connect_async_with_connector(
    request: Request,
    config: Option<WebSocketConfig>,
    disable_nagle: bool,
    _connector: Option<tokio_tungstenite::Connector>,
) -> Result<
    (
        tokio_tungstenite::WebSocketStream<MaybeTlsStream<TcpStream>>,
        Response,
    ),
    tungstenite::Error,
> {
    use tungstenite::{client::uri_mode, error::UrlError, stream::Mode};

    match uri_mode(request.uri())? {
        Mode::Plain => connect_async_with_config(request, config, disable_nagle).await,
        Mode::Tls => Err(tungstenite::Error::Url(UrlError::TlsFeatureNotEnabled)),
    }
}

// A TLS feature of this crate is enabled. See `connect_async_with_connector` above.
#[cfg(any(
    feature = "native-tls",
    feature = "native-tls-vendored",
    feature = "rustls-tls-native-roots",
    feature = "rustls-tls-webpki-roots"
))]
#[allow(
    clippy::result_large_err,
    reason = "large error is inherent to tungstenite"
)]
async fn client_async_with_connector<S>(
    request: Request,
    stream: S,
    config: Option<WebSocketConfig>,
    connector: Option<tokio_tungstenite::Connector>,
) -> Result<
    (
        tokio_tungstenite::WebSocketStream<MaybeTlsStream<S>>,
        Response,
    ),
    tungstenite::Error,
>
where
    S: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    tokio_tungstenite::client_async_tls_with_config(request, stream, config, connector).await
}

// No TLS feature of this crate is enabled. This function reproduces what
// `tokio_tungstenite::client_async_tls_with_config` does when `tokio-tungstenite` itself has no TLS
// feature: `Mode::Plain` wraps the stream as `MaybeTlsStream::Plain`, and `Mode::Tls` is an error.
// See `connect_async_with_connector` above for why it rejects `Mode::Tls`.
#[cfg(not(any(
    feature = "native-tls",
    feature = "native-tls-vendored",
    feature = "rustls-tls-native-roots",
    feature = "rustls-tls-webpki-roots"
)))]
#[allow(
    clippy::result_large_err,
    reason = "large error is inherent to tungstenite"
)]
async fn client_async_with_connector<S>(
    request: Request,
    stream: S,
    config: Option<WebSocketConfig>,
    _connector: Option<tokio_tungstenite::Connector>,
) -> Result<
    (
        tokio_tungstenite::WebSocketStream<MaybeTlsStream<S>>,
        Response,
    ),
    tungstenite::Error,
>
where
    S: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    use tungstenite::{client::uri_mode, error::UrlError, stream::Mode};

    match uri_mode(request.uri())? {
        Mode::Plain => {
            client_async_with_config(request, MaybeTlsStream::Plain(stream), config).await
        }
        Mode::Tls => Err(tungstenite::Error::Url(UrlError::TlsFeatureNotEnabled)),
    }
}

// Mirrors `tungstenite`'s own fallible request conversion; the large error is inherent.
#[allow(
    clippy::result_large_err,
    reason = "large error is inherent to tungstenite request conversion"
)]
fn map_amqp_websocket_request(req: impl IntoClientRequest) -> Result<Request, tungstenite::Error> {
    let mut request = req.into_client_request()?;

    // Sec-WebSocket-Protocol HTTP header
    //
    // Identifies the WebSocket subprotocol. For this AMQP WebSocket binding, the value MUST be
    // set to the US- ASCII text string “amqp” which refers to the 1.0 version of the AMQP 1.0
    // or greater, with version negotiation as defined by AMQP 1.0.
    request
        .headers_mut()
        .insert(SEC_WEBSOCKET_PROTOCOL, HeaderValue::from_static(super::SEC_WEBSOCKET_PROTOCOL_AMQP));

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
        .ok_or(Error::MissingSecWebSocketProtocol)?
        .map_err(|e| Error::Tungstenite(e.into()))?
    {
        "amqp" => Ok(response),
        _ => Err(Error::SecWebSocketProtocolIsNotAmqp),
    }
}
