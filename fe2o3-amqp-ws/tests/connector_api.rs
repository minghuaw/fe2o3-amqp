//! The connector-taking connect functions must exist in every feature build of this crate.
//!
//! A library crate depends on `fe2o3-amqp-ws` with no TLS feature and lets the application select
//! the TLS stack, so these functions must never be feature gated. See issue
//! <https://github.com/minghuaw/fe2o3-amqp/issues/356>.
//!
//! The dev-dependency on `tokio-tungstenite` turns on the `rustls-tls-webpki-roots` feature of
//! `tokio-tungstenite` for the test build. Cargo unifies that feature with the feature set of the
//! same dependency in the library build, so the tests below run in a build where
//! `tokio-tungstenite` has a TLS stack and this crate does not. That is the second case in issue
//! #356, and `rustls_variant_is_visible` fails to compile if the unification ever stops.
//!
//! CI compiles and runs this file with `cargo test -p fe2o3-amqp-ws --no-default-features`.

#![cfg(not(target_arch = "wasm32"))]

use fe2o3_amqp_ws::{
    native::TokioWebSocketStream,
    tokio_tungstenite::{Connector, MaybeTlsStream},
    Error, WebSocketStream,
};
use tokio::net::TcpStream;

/// The snippet from issue #356. It must resolve with no TLS feature enabled.
#[allow(dead_code)]
async fn issue_356_snippet_resolves() {
    let _ = WebSocketStream::connect_tls_with_config("wss://example.com", None, false, None).await;
}

#[allow(dead_code)]
async fn an_explicit_connector_is_accepted() {
    let _ = WebSocketStream::connect_tls_with_config(
        "wss://example.com",
        None,
        false,
        Some(Connector::Plain),
    )
    .await;
}

/// The two stream forms must also resolve with no TLS feature enabled.
///
/// This function is never called, so it opens no connection. A function pointer to either form
/// does not compile, because the `addr: impl AsRef<str>` parameter makes the function item
/// generic and the type cannot be inferred, so the test calls them instead.
#[allow(dead_code)]
async fn the_stream_forms_resolve(stream: TcpStream, another_stream: TcpStream) {
    let _: Result<WebSocketStream<TokioWebSocketStream<MaybeTlsStream<TcpStream>>>, Error> =
        WebSocketStream::connect_tls_with_stream("wss://example.com", stream).await;
    let _: Result<WebSocketStream<TokioWebSocketStream<MaybeTlsStream<TcpStream>>>, Error> =
        WebSocketStream::connect_tls_with_stream_and_config(
            "wss://example.com",
            another_stream,
            None,
            Some(Connector::Plain),
        )
        .await;
}

/// The `Connector::Rustls` variant exists only when `tokio-tungstenite` has a `rustls` feature
/// enabled. This function therefore fails to compile if the dev-dependency stops unifying that
/// feature into the test build, which would make the tests below stop guarding issue #356.
#[allow(dead_code)]
fn rustls_variant_is_visible(connector: &Connector) -> bool {
    matches!(connector, Connector::Rustls(_))
}

/// A `wss://` address must fail with `TlsFeatureNotEnabled` when this crate has no TLS stack. It
/// must never fall back to a plaintext connection, and it must not open a socket.
///
/// `tokio-tungstenite` has a TLS stack in this build, so this test also guards the second case in
/// issue #356: a TLS feature that another crate enables must not make this crate connect with a
/// stack that it never asked for.
///
/// The test is compiled out when a TLS feature of this crate is enabled, because the call would
/// then reach the network.
#[cfg(not(any(
    feature = "native-tls",
    feature = "native-tls-vendored",
    feature = "rustls-tls-native-roots",
    feature = "rustls-tls-webpki-roots"
)))]
#[tokio::test]
async fn wss_without_a_tls_feature_is_an_error() {
    use fe2o3_amqp_ws::tungstenite::{error::UrlError, Error as WsError};

    // Port 1 on the loopback address refuses a connection at once. If a call ever forwards to
    // `tokio_tungstenite` instead of failing first, the test fails fast with an IO error rather
    // than hanging.
    //
    // `None` is the value that lets `tokio_tungstenite` select the TLS stack, and
    // `Some(Connector::Plain)` is an explicit request for no TLS. Neither may reach a peer.
    for connector in [None, Some(Connector::Plain)] {
        let result =
            WebSocketStream::connect_tls_with_config("wss://127.0.0.1:1", None, false, connector)
                .await;

        match result {
            Err(fe2o3_amqp_ws::Error::Tungstenite(WsError::Url(
                UrlError::TlsFeatureNotEnabled,
            ))) => {}
            Err(other) => panic!("expected TlsFeatureNotEnabled, got {other}"),
            Ok(_) => panic!("expected TlsFeatureNotEnabled, got a connection"),
        }
    }
}

/// A `ws://` address must still connect in plaintext when this crate has no TLS stack, and the
/// connector must be ignored, which is what `tokio_tungstenite` also does.
#[cfg(not(any(
    feature = "native-tls",
    feature = "native-tls-vendored",
    feature = "rustls-tls-native-roots",
    feature = "rustls-tls-webpki-roots"
)))]
#[tokio::test]
async fn ws_without_a_tls_feature_reaches_the_socket() {
    use fe2o3_amqp_ws::tungstenite::{error::UrlError, Error as WsError};

    let result = WebSocketStream::connect_tls_with_config(
        "ws://127.0.0.1:1",
        None,
        false,
        Some(Connector::Plain),
    )
    .await;

    // The connection is refused, so this is an IO error. The point is that it is NOT
    // `TlsFeatureNotEnabled`: a `ws://` address is forwarded, not rejected.
    match result {
        Err(fe2o3_amqp_ws::Error::Tungstenite(WsError::Url(UrlError::TlsFeatureNotEnabled))) => {
            panic!("a ws:// address must not be rejected as a TLS request")
        }
        Err(_) => {}
        Ok(_) => panic!("expected the connection to be refused"),
    }
}
