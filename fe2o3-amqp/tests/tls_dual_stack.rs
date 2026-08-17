//! Verifies that `amqps://` works when both TLS features are enabled.
//!
//! With both `"rustls"` and `"native-tls"` enabled, the `()`-builder default for
//! `amqps://` is the `native-tls` connector (matching reqwest), instead of failing
//! with `TlsConnectorNotFound`. This smoke test asserts that the default path is
//! taken: the connection proceeds into the native-tls handshake (which fails against
//! a dead duplex) rather than erroring with `TlsConnectorNotFound`.

#![cfg(all(feature = "rustls", feature = "native-tls"))]

use std::time::Duration;

use fe2o3_amqp::connection::{Connection, OpenError};

#[tokio::test]
async fn amqps_with_both_tls_features_uses_native_tls_default() {
    let (client_io, server_io) = tokio::io::duplex(1024);
    // Closing the server end makes the client's TLS handshake fail immediately.
    drop(server_io);

    let err = tokio::time::timeout(
        Duration::from_secs(10),
        Connection::builder()
            .container_id("test")
            .scheme("amqps")
            .domain("localhost")
            .open_with_stream(client_io),
    )
    .await
    .expect("timed out")
    .expect_err("expected the TLS handshake to fail");

    assert!(
        !matches!(err, OpenError::TlsConnectorNotFound),
        "both-enabled amqps must not error with TlsConnectorNotFound"
    );
}
