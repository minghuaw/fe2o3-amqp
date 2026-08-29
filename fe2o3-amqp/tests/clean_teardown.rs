//! Tests that a locally initiated end/close (with or without an error) does
//! not surface as an error on the session/connection handle.
//!
//! These run over an in-memory `tokio::io::duplex` stream, so no broker or
//! network is required.

#![cfg(feature = "acceptor")]

use std::time::Duration;

use fe2o3_amqp::{
    acceptor::{ConnectionAcceptor, SessionAcceptor},
    connection::{Connection, ConnectionHandle},
    session::{Session, SessionHandle},
    types::definitions::{self, AmqpError},
};

mod common;

fn test_error() -> definitions::Error {
    definitions::Error::new(
        AmqpError::InternalError,
        Some("test error".to_string()),
        None,
    )
}

async fn establish_connection_pair() -> (
    fe2o3_amqp::acceptor::ListenerConnectionHandle,
    ConnectionHandle<()>,
) {
    let (client_io, server_io) = tokio::io::duplex(4096);

    let acceptor = ConnectionAcceptor::builder()
        .container_id("test-listener")
        .build();
    let connection_task = tokio::spawn(async move { acceptor.accept(server_io).await });

    let client_connection = common::expect_ok!(Connection::builder()
        .container_id("test-client")
        .open_with_stream(client_io))
    .await;

    let server_connection = connection_task
        .await
        .expect("connection accept task panicked")
        .expect("connection accept failed");

    (server_connection, client_connection)
}

async fn establish_session_pair(
    server_connection: &mut fe2o3_amqp::acceptor::ListenerConnectionHandle,
    client_connection: &mut ConnectionHandle<()>,
) -> SessionHandle<()> {
    let session_acceptor = SessionAcceptor::new();
    let begin_fut = common::expect_ok!(Session::begin(client_connection));
    let (session_result, begin_result) =
        tokio::join!(session_acceptor.accept(server_connection), begin_fut);
    let client_session = begin_result;
    session_result.expect("session accept failed");
    client_session
}

/// A locally initiated session end must not surface as an error on the
/// session handle.
#[tokio::test]
async fn local_session_end_returns_ok() {
    let (mut server_connection, mut client_connection) = establish_connection_pair().await;
    let mut client_session =
        establish_session_pair(&mut server_connection, &mut client_connection).await;

    let result = tokio::time::timeout(Duration::from_secs(10), client_session.end())
        .await
        .expect("end timed out");

    assert!(result.is_ok(), "local end must not error, got {:?}", result);
}

/// A locally initiated session end with an error must not surface as an error
/// on the session handle (the error is delivered to the remote).
#[tokio::test]
async fn local_session_end_with_error_returns_ok() {
    let (mut server_connection, mut client_connection) = establish_connection_pair().await;
    let mut client_session =
        establish_session_pair(&mut server_connection, &mut client_connection).await;

    let result = tokio::time::timeout(
        Duration::from_secs(10),
        client_session.end_with_error(test_error()),
    )
    .await
    .expect("end timed out");

    assert!(
        result.is_ok(),
        "local end with error must not error, got {:?}",
        result
    );
}

/// A locally initiated connection close must not surface as an error on the
/// connection handle.
#[tokio::test]
async fn local_connection_close_returns_ok() {
    let (_server_connection, mut client_connection) = establish_connection_pair().await;

    let result = tokio::time::timeout(Duration::from_secs(10), client_connection.close())
        .await
        .expect("close timed out");

    assert!(
        result.is_ok(),
        "local close must not error, got {:?}",
        result
    );
}

/// A locally initiated connection close with an error must not surface as an
/// error on the connection handle (the error is delivered to the remote).
#[tokio::test]
async fn local_connection_close_with_error_returns_ok() {
    let (_server_connection, mut client_connection) = establish_connection_pair().await;

    let result = tokio::time::timeout(
        Duration::from_secs(10),
        client_connection.close_with_error(test_error()),
    )
    .await
    .expect("close timed out");

    assert!(
        result.is_ok(),
        "local close with error must not error, got {:?}",
        result
    );
}
