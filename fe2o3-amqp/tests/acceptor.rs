//! Tests for the acceptor flow (connection, session, and link acceptors)
//! over in-memory `tokio::io::duplex` streams, so no broker or network is
//! required.

#![cfg(feature = "acceptor")]

use std::time::Duration;

use fe2o3_amqp::{
    acceptor::{
        error::AcceptorAttachError,
        {ConnectionAcceptor, LinkAcceptor, SessionAcceptor},
    },
    connection::{Connection, ConnectionStopReason},
    link::SessionStopReason,
    Session,
};

/// When the connection stops while the listener session is waiting for an
/// incoming link, `LinkAcceptor::accept` must surface the connection-level
/// stop reason rather than a fabricated one.
#[tokio::test]
async fn listener_acceptor_reports_connection_closed() {
    let (client_io, server_io) = tokio::io::duplex(4096);

    let acceptor = ConnectionAcceptor::builder()
        .container_id("test-listener")
        .build();
    let connection_task = tokio::spawn(async move { acceptor.accept(server_io).await });

    let mut client_connection = Connection::builder()
        .container_id("test-client")
        .open_with_stream(client_io)
        .await
        .expect("client connection failed");

    let mut server_connection = connection_task
        .await
        .expect("connection accept task panicked")
        .expect("connection accept failed");

    // The listener session is accepted concurrently with the client beginning
    // its session; the connection handle stays in scope so the connection
    // stays alive.
    let session_acceptor = SessionAcceptor::new();
    let (session_result, begin_result) = tokio::join!(
        session_acceptor.accept(&mut server_connection),
        Session::begin(&mut client_connection),
    );
    let _client_session = begin_result.expect("client session begin failed");
    let mut listener_session = session_result.expect("session accept failed");

    let link_acceptor = LinkAcceptor::builder().build();
    let accept_task =
        tokio::spawn(async move { link_acceptor.accept(&mut listener_session).await });

    // Stopping the connection must surface as `ConnectionClosed` on the
    // acceptor side.
    drop(client_connection);

    let result = tokio::time::timeout(Duration::from_secs(10), accept_task)
        .await
        .expect("accept task timed out")
        .expect("accept task panicked");

    match result {
        Err(AcceptorAttachError::SessionStopped(SessionStopReason::ConnectionStopped(
            ConnectionStopReason::RemoteClosed,
        ))) => {}
        other => panic!("expected SessionStopped(ConnectionClosed), got {:?}", other),
    }
}

/// When only the session ends (the remote sends `End`) while the connection
/// stays alive, `LinkAcceptor::accept` must report a session-level stop
/// rather than a connection-level one.
#[tokio::test]
async fn listener_acceptor_reports_session_ended() {
    let (client_io, server_io) = tokio::io::duplex(4096);

    let acceptor = ConnectionAcceptor::builder()
        .container_id("test-listener")
        .build();
    let connection_task = tokio::spawn(async move { acceptor.accept(server_io).await });

    let mut client_connection = Connection::builder()
        .container_id("test-client")
        .open_with_stream(client_io)
        .await
        .expect("client connection failed");

    let mut server_connection = connection_task
        .await
        .expect("connection accept task panicked")
        .expect("connection accept failed");

    // The listener session is accepted concurrently with the client beginning
    // its session; the connection handle stays in scope so the connection
    // stays alive.
    let session_acceptor = SessionAcceptor::new();
    let (session_result, begin_result) = tokio::join!(
        session_acceptor.accept(&mut server_connection),
        Session::begin(&mut client_connection),
    );
    let client_session = begin_result.expect("client session begin failed");
    let mut listener_session = session_result.expect("session accept failed");

    let link_acceptor = LinkAcceptor::builder().build();
    let accept_task =
        tokio::spawn(async move { link_acceptor.accept(&mut listener_session).await });

    // Ending only the session must surface as `Ended`, with the connection
    // still alive.
    drop(client_session);

    let result = tokio::time::timeout(Duration::from_secs(10), accept_task)
        .await
        .expect("accept task timed out")
        .expect("accept task panicked");

    match result {
        Err(AcceptorAttachError::SessionStopped(SessionStopReason::RemoteEnded)) => {}
        other => panic!("expected SessionStopped(RemoteEnded), got {:?}", other),
    }
}
