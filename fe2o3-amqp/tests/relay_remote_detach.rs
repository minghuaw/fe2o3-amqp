//! Tests of the relay-handled remote link detach/close semantics and of the
//! link drop (Drop edge) behavior:
//!
//! - a remote closing detach is answered by the session relay at arrival time
//!   (the link engine never writes a response), and the relay fails the
//!   deliveries that are still pending on the link;
//! - dropping a sender link without a clean close leaves its pending
//!   deliveries unresolved (they are still settled by the peer's dispositions
//!   through the relay) unless the delivery can no longer be settled.
//!
//! These run over an in-memory `tokio::io::duplex` stream, so no broker or
//! network is required.

#![cfg(feature = "acceptor")]

use std::time::Duration;

use fe2o3_amqp::{
    acceptor::{
        ConnectionAcceptor, LinkAcceptor, LinkEndpoint, ListenerSessionHandle, SessionAcceptor,
    },
    connection::{Connection, ConnectionHandle},
    link::{LinkStateError, SendError},
    session::{Session, SessionHandle},
    types::messaging::Message,
    Sendable, Sender,
};

mod common;

async fn establish_connection_pair() -> (
    fe2o3_amqp::acceptor::ListenerConnectionHandle,
    ConnectionHandle<()>,
) {
    let (client_io, server_io) = tokio::io::duplex(64 * 1024);

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
) -> (ListenerSessionHandle, SessionHandle<()>) {
    let session_acceptor = SessionAcceptor::new();
    let begin_fut = common::expect_ok!(Session::begin(client_connection));
    let (session_result, begin_result) =
        tokio::join!(session_acceptor.accept(server_connection), begin_fut);
    let client_session = begin_result;
    let listener_session = session_result.expect("session accept failed");
    (listener_session, client_session)
}

/// Attach the client sender concurrently with the acceptor accepting the
/// incoming link, returning both ends.
async fn establish_link_pair(
    listener_session: &mut ListenerSessionHandle,
    client_session: &mut SessionHandle<()>,
    name: &str,
) -> (Sender, fe2o3_amqp::Receiver) {
    let link_acceptor = LinkAcceptor::builder().build();
    let attach_fut = common::expect_ok!(Sender::attach(client_session, name, "test-queue"));
    let (accept_result, attach_result) =
        tokio::join!(link_acceptor.accept(listener_session), attach_fut);

    let sender = attach_result;
    let receiver = match accept_result.expect("link accept failed") {
        LinkEndpoint::Receiver(receiver) => receiver,
        other => panic!("expected receiver endpoint, got {:?}", other),
    };
    (sender, receiver)
}

/// Dropping a sender link without a clean close must not, by itself, fail the
/// deliveries that are still pending on it. Once the peer's close response
/// (echo) comes back and finds the link engine gone, the relay fails the
/// stranded delivery instead of leaving it pending until the teardown.
#[tokio::test]
async fn pending_delivery_failed_by_close_echo_after_sender_dropped_without_close() {
    let (mut server_connection, mut client_connection) = establish_connection_pair().await;
    let (mut listener_session, mut client_session) =
        establish_session_pair(&mut server_connection, &mut client_connection).await;
    let (mut sender, _receiver) = establish_link_pair(
        &mut listener_session,
        &mut client_session,
        "sender-drop-1",
    )
    .await;

    let delivery = sender
        .send_batchable(Sendable::builder().message("hello").build())
        .await
        .expect("send failed");
    drop(sender);

    // The drop itself does not touch the pending delivery; the peer's echo of
    // the drop's closing detach fails it (the engine is gone, so only the
    // relay can fail it).
    let result = tokio::time::timeout(Duration::from_secs(10), delivery)
        .await
        .expect("delivery did not resolve after the close echo")
        .expect_err("stranded delivery must fail on the close echo");
    match result {
        SendError::LinkStateError(LinkStateError::RemoteClosed) => {}
        other => panic!("expected RemoteClosed, got {:?}", other),
    }

    // The session is still healthy: it must end cleanly.
    client_session
        .end()
        .await
        .expect("session end failed");
    client_connection.close().await.expect("close failed");
}

/// A remote-initiated closing detach must fail the deliveries that are still
/// pending on the link with `RemoteClosed`, even when the link engine never
/// processes the detach (it is answered by the relay). Dropping the sender
/// afterwards must not disturb the sessions: a fresh link pair on the same
/// sessions still works.
#[tokio::test]
async fn remote_link_close_fails_pending_delivery_and_session_survives() {
    let (mut server_connection, mut client_connection) = establish_connection_pair().await;
    let (mut listener_session, mut client_session) =
        establish_session_pair(&mut server_connection, &mut client_connection).await;
    let (mut sender, receiver) = establish_link_pair(
        &mut listener_session,
        &mut client_session,
        "remote-close-1",
    )
    .await;

    let delivery = sender
        .send_batchable(Sendable::builder().message("hello").build())
        .await
        .expect("send failed");

    // The remote closes the link while the delivery is still unsettled.
    receiver.close().await.expect("receiver close failed");

    // The relay fails the pending delivery when answering the closing detach.
    let result = tokio::time::timeout(Duration::from_secs(10), delivery)
        .await
        .expect("delivery did not resolve")
        .expect_err("pending delivery must fail on remote close");
    match result {
        SendError::LinkStateError(LinkStateError::RemoteClosed) => {}
        other => panic!("expected RemoteClosed, got {:?}", other),
    }

    // Dropping the sender now must not send a duplicate closing detach or
    // otherwise disturb the sessions.
    drop(sender);

    // The sessions must still be healthy: a fresh link pair round trips.
    let (mut sender2, mut receiver2) = establish_link_pair(
        &mut listener_session,
        &mut client_session,
        "remote-close-2",
    )
    .await;

    let message = Message::from("still-alive");
    let send_task = tokio::spawn(async move {
        let outcome = sender2.send(message).await.unwrap();
        outcome.accepted_or("Not accepted").unwrap();
        sender2.close().await.unwrap();
    });

    let received = tokio::time::timeout(Duration::from_secs(10), receiver2.recv::<String>())
        .await
        .expect("timed out waiting for message")
        .expect("recv failed");
    receiver2.accept(&received).await.unwrap();
    assert_eq!(received.body(), "still-alive");
    receiver2.close().await.unwrap();
    send_task.await.unwrap();

    client_session.close().await.unwrap();
    client_connection.close().await.unwrap();
}
