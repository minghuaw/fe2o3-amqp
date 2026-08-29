//! Tests that link operations surface the session/connection stop reason.
//!
//! These run over an in-memory `tokio::io::duplex` stream, so no broker or
//! network is required.

#![cfg(feature = "acceptor")]

use std::time::Duration;

use fe2o3_amqp::{
    acceptor::{
        ConnectionAcceptor, LinkAcceptor, ListenerConnectionHandle, ListenerSessionHandle,
        SessionAcceptor,
    },
    connection::{Connection, ConnectionHandle, ConnectionStopReason},
    link::{LinkStateError, RecvError, SendError, SenderAttachError, SessionStopReason},
    session::{Session, SessionHandle},
    types::{
        definitions::{self, AmqpError},
        messaging::{Source, Target},
    },
    Receiver, Sendable, Sender,
};

mod common;

fn test_error() -> definitions::Error {
    definitions::Error::new(
        AmqpError::InternalError,
        Some("test error".to_string()),
        None,
    )
}

async fn establish_connection_pair() -> (ListenerConnectionHandle, ConnectionHandle<()>) {
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

/// The listener session is accepted concurrently with the client beginning its
/// session; the connection handles stay in scope so the connections stay alive.
async fn establish_session_pair(
    server_connection: &mut ListenerConnectionHandle,
    client_connection: &mut ConnectionHandle<()>,
) -> (SessionHandle<()>, ListenerSessionHandle) {
    let session_acceptor = SessionAcceptor::new();
    let begin_fut = common::expect_ok!(Session::begin(client_connection));
    let (session_result, begin_result) =
        tokio::join!(session_acceptor.accept(server_connection), begin_fut);
    let client_session = begin_result;
    let listener_session = session_result.expect("session accept failed");
    (client_session, listener_session)
}

/// Attach a client sender link, with the listener accepting it concurrently.
async fn attach_sender(
    client_session: &mut SessionHandle<()>,
    listener_session: &mut ListenerSessionHandle,
    name: &str,
) -> Sender {
    let link_acceptor = LinkAcceptor::new();
    let attach_fut = common::expect_ok!(Sender::builder()
        .name(name)
        .source(Source::builder().build())
        .target(Target::builder().build())
        .attach(client_session));
    let (link_result, attach_result) =
        tokio::join!(link_acceptor.accept(listener_session), attach_fut);
    let _server_receiver = link_result.expect("link accept failed");
    attach_result
}

/// Attach a client receiver link, with the listener accepting it concurrently.
async fn attach_receiver(
    client_session: &mut SessionHandle<()>,
    listener_session: &mut ListenerSessionHandle,
    name: &str,
) -> Receiver {
    let link_acceptor = LinkAcceptor::new();
    let attach_fut = common::expect_ok!(Receiver::builder()
        .name(name)
        .source(Source::builder().build())
        .target(Target::builder().build())
        .attach(client_session));
    let (link_result, attach_result) =
        tokio::join!(link_acceptor.accept(listener_session), attach_fut);
    let _server_sender = link_result.expect("link accept failed");
    attach_result
}

/// Sends until the teardown has propagated and the send surfaces the expected
/// stop reason.
///
/// The teardown is asynchronous: a send may win the race and be settled by the
/// still-alive remote before the session/connection engine exits. Once the
/// engine exits, every send fails with the stop reason, so retrying is
/// deterministic.
async fn expect_send_stop_reason(sender: &mut Sender, expected: SessionStopReason) {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let result = sender
            .send(Sendable::builder().message("hello").settled(true).build())
            .await;
        match result {
            Ok(_) => {
                // Teardown has not propagated yet; retry.
                assert!(
                    std::time::Instant::now() < deadline,
                    "timed out waiting for the stop reason to propagate"
                );
            }
            Err(SendError::LinkStateError(LinkStateError::SessionStopped(reason))) => {
                assert_eq!(reason, expected, "unexpected stop reason");
                return;
            }
            Err(other) => panic!("unexpected send error: {:?}", other),
        }
    }
}

/// When the connection stops, a send on an attached sender link must surface
/// the connection-level stop reason.
#[tokio::test]
async fn link_send_surfaces_connection_closed() {
    let (mut server_connection, mut client_connection) = establish_connection_pair().await;
    let (mut client_session, mut listener_session) =
        establish_session_pair(&mut server_connection, &mut client_connection).await;
    let mut sender = attach_sender(&mut client_session, &mut listener_session, "sender-1").await;

    drop(client_connection);

    expect_send_stop_reason(
        &mut sender,
        SessionStopReason::ConnectionStopped(ConnectionStopReason::Closed),
    )
    .await;
}

/// When only the session ends while the connection stays alive, a send on an
/// attached sender link must surface the session-level stop reason.
#[tokio::test]
async fn link_send_surfaces_session_ended() {
    let (mut server_connection, mut client_connection) = establish_connection_pair().await;
    let (mut client_session, mut listener_session) =
        establish_session_pair(&mut server_connection, &mut client_connection).await;
    let mut sender = attach_sender(&mut client_session, &mut listener_session, "sender-1").await;

    drop(client_session);

    expect_send_stop_reason(&mut sender, SessionStopReason::Ended).await;
}

/// When the connection stops, a receive on an attached receiver link must
/// surface the connection-level stop reason.
#[tokio::test]
async fn link_recv_surfaces_connection_closed() {
    let (mut server_connection, mut client_connection) = establish_connection_pair().await;
    let (mut client_session, mut listener_session) =
        establish_session_pair(&mut server_connection, &mut client_connection).await;
    let mut receiver =
        attach_receiver(&mut client_session, &mut listener_session, "receiver-1").await;

    drop(server_connection);

    let result = tokio::time::timeout(Duration::from_secs(10), receiver.recv::<String>())
        .await
        .expect("recv timed out");

    match result {
        Err(RecvError::LinkStateError(LinkStateError::SessionStopped(
            SessionStopReason::ConnectionStopped(ConnectionStopReason::RemoteClosed),
        ))) => {}
        other => panic!("expected SessionStopped(ConnectionClosed), got {:?}", other),
    }
}

/// When only the session ends while the connection stays alive, a receive on
/// an attached receiver link must surface the session-level stop reason.
#[tokio::test]
async fn link_recv_surfaces_session_ended() {
    let (mut server_connection, mut client_connection) = establish_connection_pair().await;
    let (mut client_session, mut listener_session) =
        establish_session_pair(&mut server_connection, &mut client_connection).await;
    let mut receiver =
        attach_receiver(&mut client_session, &mut listener_session, "receiver-1").await;

    drop(listener_session);

    let result = tokio::time::timeout(Duration::from_secs(10), receiver.recv::<String>())
        .await
        .expect("recv timed out");

    match result {
        Err(RecvError::LinkStateError(LinkStateError::SessionStopped(
            SessionStopReason::RemoteEnded,
        ))) => {}
        other => panic!("expected SessionStopped(RemoteEnded), got {:?}", other),
    }
}

/// Attaching a new link after the connection stopped must surface the
/// connection-level stop reason.
#[tokio::test]
async fn attach_after_stop_reports_stop_reason() {
    let (mut server_connection, mut client_connection) = establish_connection_pair().await;
    let (mut client_session, _listener_session) =
        establish_session_pair(&mut server_connection, &mut client_connection).await;

    drop(server_connection);

    let result = tokio::time::timeout(
        Duration::from_secs(10),
        Sender::builder()
            .name("sender-2")
            .source(Source::builder().build())
            .target(Target::builder().build())
            .attach(&mut client_session),
    )
    .await
    .expect("attach timed out");

    match result {
        Err(SenderAttachError::SessionStopped(SessionStopReason::ConnectionStopped(
            ConnectionStopReason::RemoteClosed,
        ))) => {}
        other => panic!("expected SessionStopped(ConnectionClosed), got {:?}", other),
    }
}

/// A locally ended session with an error must surface as `EndedWithError`
/// with the local error.
#[tokio::test]
async fn link_send_surfaces_local_end_with_error() {
    let (mut server_connection, mut client_connection) = establish_connection_pair().await;
    let (mut client_session, mut listener_session) =
        establish_session_pair(&mut server_connection, &mut client_connection).await;
    let mut sender = attach_sender(&mut client_session, &mut listener_session, "sender-1").await;

    let error = test_error();
    client_session
        .end_with_error(error.clone())
        .await
        .expect("end failed");

    expect_send_stop_reason(&mut sender, SessionStopReason::EndedWithError(error)).await;
}

/// A remote-initiated session end with an error must surface as
/// `RemoteEndedWithError` with the remote error.
#[tokio::test]
async fn link_send_surfaces_remote_end_with_error() {
    let (mut server_connection, mut client_connection) = establish_connection_pair().await;
    let (mut client_session, mut listener_session) =
        establish_session_pair(&mut server_connection, &mut client_connection).await;
    let mut sender = attach_sender(&mut client_session, &mut listener_session, "sender-1").await;

    let error = test_error();
    listener_session
        .end_with_error(error.clone())
        .await
        .expect("end failed");

    expect_send_stop_reason(&mut sender, SessionStopReason::RemoteEndedWithError(error)).await;
}

/// A remote-initiated clean session end must surface as `RemoteEnded`.
#[tokio::test]
async fn link_send_surfaces_remote_end() {
    let (mut server_connection, mut client_connection) = establish_connection_pair().await;
    let (mut client_session, mut listener_session) =
        establish_session_pair(&mut server_connection, &mut client_connection).await;
    let mut sender = attach_sender(&mut client_session, &mut listener_session, "sender-1").await;

    drop(listener_session);

    expect_send_stop_reason(&mut sender, SessionStopReason::RemoteEnded).await;
}

/// A locally closed connection with an error must surface as
/// `ConnectionStopped(ClosedWithError(..))` with the local error, regardless
/// of how the peer responds.
#[tokio::test]
async fn link_send_surfaces_local_close_with_error() {
    let (mut server_connection, mut client_connection) = establish_connection_pair().await;
    let (mut client_session, mut listener_session) =
        establish_session_pair(&mut server_connection, &mut client_connection).await;
    let mut sender = attach_sender(&mut client_session, &mut listener_session, "sender-1").await;

    let error = test_error();
    client_connection
        .close_with_error(error.clone())
        .await
        .expect("close failed");

    expect_send_stop_reason(
        &mut sender,
        SessionStopReason::ConnectionStopped(ConnectionStopReason::ClosedWithError(error)),
    )
    .await;
}

/// A remote-initiated connection close with an error must surface as
/// `ConnectionStopped(RemoteClosedWithError(..))` with the remote error.
#[tokio::test]
async fn link_send_surfaces_remote_close_with_error() {
    let (mut server_connection, mut client_connection) = establish_connection_pair().await;
    let (mut client_session, mut listener_session) =
        establish_session_pair(&mut server_connection, &mut client_connection).await;
    let mut sender = attach_sender(&mut client_session, &mut listener_session, "sender-1").await;

    let error = test_error();
    server_connection
        .close_with_error(error.clone())
        .await
        .expect("close failed");

    expect_send_stop_reason(
        &mut sender,
        SessionStopReason::ConnectionStopped(ConnectionStopReason::RemoteClosedWithError(error)),
    )
    .await;
}
