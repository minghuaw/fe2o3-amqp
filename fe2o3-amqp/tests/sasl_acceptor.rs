//! Tests for the server-side SASL negotiation of [`ConnectionAcceptor`].
//!
//! These run over an in-memory `tokio::io::duplex` stream, so no broker or
//! network is required.

#![cfg(feature = "acceptor")]

use fe2o3_amqp::{
    acceptor::{ConnectionAcceptor, SaslPlainMechanism},
    connection::{Connection, OpenError},
    sasl_profile::SaslProfile,
};
use fe2o3_amqp_types::sasl::SaslCode;

/// A failed SASL outcome (here, a wrong password) must surface as
/// `OpenError::SaslError` on the acceptor side rather than letting the
/// connection proceed to AMQP negotiation as if authentication had succeeded.
#[tokio::test]
async fn sasl_plain_auth_failure_returns_sasl_error() {
    let (client_io, server_io) = tokio::io::duplex(4096);

    let acceptor = ConnectionAcceptor::builder()
        .container_id("test-listener")
        .sasl_acceptor(SaslPlainMechanism::new("guest", "guest"))
        .build();

    let server = tokio::spawn(async move { acceptor.accept(server_io).await });

    // Client authenticates with the wrong password.
    let client = tokio::spawn(async move {
        Connection::builder()
            .container_id("test-client")
            .sasl_profile(SaslProfile::Plain {
                username: "guest".to_string(),
                password: "wrong-password".to_string(),
            })
            .open_with_stream(client_io)
            .await
    });

    let server_result = server.await.expect("server task panicked");
    // The client is expected to fail as well; we only care that it terminates.
    let _ = client.await.expect("client task panicked");

    match server_result {
        Err(OpenError::SaslError { code, .. }) => {
            assert_eq!(code, SaslCode::Auth, "expected SASL auth failure code");
        }
        other => panic!("expected OpenError::SaslError, got {:?}", other),
    }
}

/// Sanity check that the happy path still works: with valid credentials the
/// acceptor completes SASL and AMQP negotiation and returns a connection
/// handle.
#[tokio::test]
async fn sasl_plain_auth_success_opens_connection() {
    let (client_io, server_io) = tokio::io::duplex(4096);

    let acceptor = ConnectionAcceptor::builder()
        .container_id("test-listener")
        .sasl_acceptor(SaslPlainMechanism::new("guest", "guest"))
        .build();

    let server = tokio::spawn(async move { acceptor.accept(server_io).await });

    let client = tokio::spawn(async move {
        Connection::builder()
            .container_id("test-client")
            .sasl_profile(SaslProfile::Plain {
                username: "guest".to_string(),
                password: "guest".to_string(),
            })
            .open_with_stream(client_io)
            .await
    });

    let server_result = server.await.expect("server task panicked");
    let client_result = client.await.expect("client task panicked");

    assert!(
        server_result.is_ok(),
        "acceptor should open the connection on valid credentials, got {:?}",
        server_result.err()
    );
    assert!(
        client_result.is_ok(),
        "client should open the connection on valid credentials, got {:?}",
        client_result.err()
    );
}
