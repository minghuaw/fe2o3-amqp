//! Tests that a locally initiated end/close (with or without an error) does
//! not surface as an error on the session/connection handle.
//!
//! These run over an in-memory `tokio::io::duplex` stream, so no broker or
//! network is required.

#![cfg(feature = "acceptor")]

use std::time::Duration;

use fe2o3_amqp::{
    acceptor::{ConnectionAcceptor, ListenerSessionHandle, SessionAcceptor},
    connection::{
        Connection, ConnectionHandle, ConnectionOutcome, Error as ConnectionError, TryCloseError,
    },
    session::{Error as SessionError, Session, SessionHandle, SessionOutcome, TryEndError},
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
) -> (SessionHandle<()>, ListenerSessionHandle) {
    let session_acceptor = SessionAcceptor::new();
    let begin_fut = common::expect_ok!(Session::begin(client_connection));
    let (session_result, begin_result) =
        tokio::join!(session_acceptor.accept(server_connection), begin_fut);
    let client_session = begin_result;
    let server_session = session_result.expect("session accept failed");
    (client_session, server_session)
}

/// A locally initiated session end must not surface as an error on the
/// session handle.
#[tokio::test]
async fn local_session_end_returns_ok() {
    let (mut server_connection, mut client_connection) = establish_connection_pair().await;
    let (mut client_session, _server_session) =
        establish_session_pair(&mut server_connection, &mut client_connection).await;

    let result = tokio::time::timeout(Duration::from_secs(10), client_session.end())
        .await
        .expect("end timed out");

    assert!(
        matches!(result, Ok(SessionOutcome::Ended)),
        "local end must report Ended, got {result:?}"
    );
}

/// A locally initiated session end with an error must not surface as an error
/// on the session handle (the error is delivered to the remote).
#[tokio::test]
async fn local_session_end_with_error_returns_ok() {
    let (mut server_connection, mut client_connection) = establish_connection_pair().await;
    let (mut client_session, _server_session) =
        establish_session_pair(&mut server_connection, &mut client_connection).await;

    let result = tokio::time::timeout(
        Duration::from_secs(10),
        client_session.end_with_error(test_error()),
    )
    .await
    .expect("end timed out");

    assert!(
        matches!(result, Ok(SessionOutcome::EndedWithError(_))),
        "local end with error must report EndedWithError, got {result:?}"
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
        matches!(result, Ok(ConnectionOutcome::Closed)),
        "local close must report Closed, got {result:?}"
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
        matches!(result, Ok(ConnectionOutcome::ClosedWithError(_))),
        "local close with error must report ClosedWithError, got {result:?}"
    );
}

/// A remote clean session end must not surface as an error on the session
/// handle.
#[tokio::test]
async fn remote_session_end_returns_ok() {
    let (mut server_connection, mut client_connection) = establish_connection_pair().await;
    let (mut client_session, mut server_session) =
        establish_session_pair(&mut server_connection, &mut client_connection).await;

    let (server_result, client_result) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(server_session.end(), client_session.on_end())
    })
    .await
    .expect("remote end timed out");

    server_result.expect("server end failed");
    assert!(
        matches!(client_result, Ok(SessionOutcome::RemoteEnded)),
        "remote clean end must report RemoteEnded, got {client_result:?}"
    );
}

/// A remote session end with an error is reported on the session handle; the
/// outcome is delivered once and later calls report `AlreadyEnded`.
#[tokio::test]
async fn remote_session_end_with_error_is_reported() {
    let (mut server_connection, mut client_connection) = establish_connection_pair().await;
    let (mut client_session, mut server_session) =
        establish_session_pair(&mut server_connection, &mut client_connection).await;

    let (server_result, client_result) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(
            server_session.end_with_error(test_error()),
            client_session.on_end()
        )
    })
    .await
    .expect("remote end timed out");

    server_result.expect("server end failed");
    let expected = test_error();
    assert!(
        matches!(&client_result, Ok(SessionOutcome::RemoteEndedWithError(error)) if error == &expected),
        "remote end with error must be reported as an outcome, got {client_result:?}"
    );

    let error = client_session
        .on_end()
        .await
        .expect_err("the outcome was already observed");
    assert!(
        matches!(error, SessionError::AlreadyEnded),
        "a later call must report AlreadyEnded, got {error:?}"
    );
}

/// Repeated local session ends report `AlreadyEnded` after the outcome was
/// observed once.
#[tokio::test]
async fn repeated_session_end_reports_already_ended() {
    let (mut server_connection, mut client_connection) = establish_connection_pair().await;
    let (mut client_session, _server_session) =
        establish_session_pair(&mut server_connection, &mut client_connection).await;

    client_session.end().await.expect("local end failed");
    let error = client_session
        .on_end()
        .await
        .expect_err("the outcome was already observed");
    assert!(matches!(error, SessionError::AlreadyEnded));
    assert!(matches!(
        client_session.try_end(),
        Err(TryEndError::AlreadyEnded)
    ));
}

/// A remote clean connection close must not surface as an error on the
/// connection handle.
#[tokio::test]
async fn remote_connection_close_returns_ok() {
    let (mut server_connection, mut client_connection) = establish_connection_pair().await;

    let (server_result, client_result) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(server_connection.close(), client_connection.on_close())
    })
    .await
    .expect("remote close timed out");

    server_result.expect("server close failed");
    assert!(
        matches!(client_result, Ok(ConnectionOutcome::RemoteClosed)),
        "remote clean close must report RemoteClosed, got {client_result:?}"
    );
}

/// A remote connection close with an error is reported on the connection
/// handle; the outcome is delivered once and later calls report `AlreadyClosed`.
#[tokio::test]
async fn remote_connection_close_with_error_is_reported() {
    let (mut server_connection, mut client_connection) = establish_connection_pair().await;

    let (server_result, client_result) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(
            server_connection.close_with_error(test_error()),
            client_connection.on_close()
        )
    })
    .await
    .expect("remote close timed out");

    server_result.expect("server close failed");
    let expected = test_error();
    assert!(
        matches!(&client_result, Ok(ConnectionOutcome::RemoteClosedWithError(error)) if error == &expected),
        "remote close with error must be reported as an outcome, got {client_result:?}"
    );

    let error = client_connection
        .on_close()
        .await
        .expect_err("the outcome was already observed");
    assert!(
        matches!(error, ConnectionError::AlreadyClosed),
        "a later call must report AlreadyClosed, got {error:?}"
    );
}

/// Repeated local connection closes report `AlreadyClosed` after the outcome
/// was observed once.
#[tokio::test]
async fn repeated_connection_close_reports_already_closed() {
    let (_server_connection, mut client_connection) = establish_connection_pair().await;

    client_connection.close().await.expect("local close failed");
    let error = client_connection
        .on_close()
        .await
        .expect_err("the outcome was already observed");
    assert!(matches!(error, ConnectionError::AlreadyClosed));
    assert!(matches!(
        client_connection.try_close(),
        Err(TryCloseError::AlreadyClosed)
    ));
}

/// A session whose connection stopped reports the connection stop as an error
/// on `on_end`, consistent with link operations failing when their session
/// stopped.
#[tokio::test]
async fn session_end_reports_connection_stop_as_error() {
    let (mut server_connection, mut client_connection) = establish_connection_pair().await;
    let (mut client_session, _server_session) =
        establish_session_pair(&mut server_connection, &mut client_connection).await;

    let (server_result, client_result) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(server_connection.close(), client_session.on_end())
    })
    .await
    .expect("session end timed out");

    server_result.expect("server close failed");
    let error = client_result.expect_err("the connection stopped first");
    assert!(
        matches!(error, fe2o3_amqp::session::Error::ConnectionStopped(_)),
        "expected ConnectionStopped, got {error:?}"
    );
}
