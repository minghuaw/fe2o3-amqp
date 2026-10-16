//! Regression tests for link resumption carrying unsettled deliveries.
//!
//! An unsettled delivery is kept in the link's unsettled map, so a
//! `send_batchable` future stays pending across a session/connection stop and
//! resolves with the peer's disposition after the link is resumed. These run
//! over an in-memory `tokio::io::duplex` stream, so no broker is required.

#![cfg(feature = "acceptor")]

use std::time::Duration;

use fe2o3_amqp::{
    acceptor::{
        ConnectionAcceptor, LinkAcceptor, LinkEndpoint, ListenerConnectionHandle,
        ListenerSessionHandle, SessionAcceptor,
    },
    connection::{Connection, ConnectionHandle},
    link::LinkOutcome,
    session::{Session, SessionHandle},
    Sendable, Sender,
};

mod common;

async fn establish_connection_pair() -> (ListenerConnectionHandle, ConnectionHandle<()>) {
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
    server_connection: &mut ListenerConnectionHandle,
    client_connection: &mut ConnectionHandle<()>,
) -> (ListenerSessionHandle, SessionHandle<()>) {
    let session_acceptor = SessionAcceptor::builder().build();
    let begin_fut = common::expect_ok!(Session::begin(client_connection));
    let (session_result, begin_result) =
        tokio::join!(session_acceptor.accept(server_connection), begin_fut);
    (session_result.expect("session accept failed"), begin_result)
}

async fn attach_sender(
    listener_session: &mut ListenerSessionHandle,
    client_session: &mut SessionHandle<()>,
) -> (Sender, fe2o3_amqp::Receiver) {
    let link_acceptor = LinkAcceptor::builder().build();
    let attach_fut =
        common::expect_ok!(Sender::attach(client_session, "test-sender", "test-queue"));
    let (accept_result, attach_result) =
        tokio::join!(link_acceptor.accept(listener_session), attach_fut);

    let sender = attach_result;
    let receiver = match accept_result.expect("link accept failed") {
        LinkEndpoint::Receiver(receiver) => receiver,
        other => panic!("expected receiver endpoint, got {:?}", other),
    };
    (sender, receiver)
}

/// Accept the reattached link, receive the re-sent delivery and accept it.
async fn accept_and_accept_delivery(
    mut listener_session: ListenerSessionHandle,
) -> (fe2o3_amqp::Delivery<String>, ListenerSessionHandle) {
    let link_acceptor = LinkAcceptor::builder().build();
    let link = link_acceptor
        .accept(&mut listener_session)
        .await
        .expect("link accept failed");
    let mut receiver = match link {
        LinkEndpoint::Receiver(receiver) => receiver,
        other => panic!("expected receiver endpoint, got {:?}", other),
    };
    let delivery = tokio::time::timeout(Duration::from_secs(10), receiver.recv::<String>())
        .await
        .expect("timed out waiting for the resumed delivery")
        .expect("recv failed");
    receiver.accept(&delivery).await.expect("accept failed");
    (delivery, listener_session)
}

/// An unsettled delivery survives the loss of its connection and resolves with
/// the peer's disposition after the link is resumed on another connection.
#[tokio::test]
async fn pending_delivery_resolves_after_resume_on_new_connection() {
    let (mut server_a, mut client_a) = establish_connection_pair().await;
    let (mut server_b, mut client_b) = establish_connection_pair().await;
    let (mut listener_a, mut session_a) =
        establish_session_pair(&mut server_a, &mut client_a).await;
    let (listener_b, mut session_b) = establish_session_pair(&mut server_b, &mut client_b).await;

    let (mut sender, _receiver_a) = attach_sender(&mut listener_a, &mut session_a).await;

    // Send unsettled without waiting for the outcome.
    let delivery = sender
        .send_batchable(Sendable::builder().message("hello").build())
        .await
        .expect("send failed");

    // Tear down connection A without a link detach; the delivery must stay
    // pending rather than fail with the stop reason.
    drop(listener_a);
    drop(session_a);
    drop(client_a);
    drop(server_a);

    let accept_b = tokio::spawn(accept_and_accept_delivery(listener_b));

    common::expect_ok!(sender.detach_then_resume_on_session(&session_b)).await;

    let (received, _listener_b) = accept_b.await.expect("accept task panicked");
    assert_eq!(received.body(), "hello");

    let outcome = common::expect_ok!(delivery).await;
    outcome.accepted_or("not accepted").unwrap();

    session_b.close().await.unwrap();
    client_b.close().await.unwrap();
}

/// An unsettled delivery survives a link detach and resolves with the peer's
/// disposition after the link is resumed on the same session.
#[tokio::test]
async fn pending_delivery_resolves_after_resume_on_same_session() {
    let (mut server_a, mut client_a) = establish_connection_pair().await;
    let (mut listener_a, mut session_a) =
        establish_session_pair(&mut server_a, &mut client_a).await;

    let (mut sender, mut receiver_a) = attach_sender(&mut listener_a, &mut session_a).await;

    let delivery = sender
        .send_batchable(Sendable::builder().message("hello").build())
        .await
        .expect("send failed");

    // Drive the acceptor receiver so it answers the sender's non-closing
    // detach.
    let recv_a = tokio::spawn(async move {
        let _ = receiver_a.recv::<String>().await;
    });
    let (detached, status) = common::expect_ok!(sender.detach()).await;
    assert!(matches!(status, LinkOutcome::Detached { .. }));
    recv_a.await.unwrap();

    let accept = tokio::spawn(accept_and_accept_delivery(listener_a));

    let sender = common::expect_ok!(detached.resume()).await;
    let (received, _listener_a) = accept.await.expect("accept task panicked");
    assert_eq!(received.body(), "hello");

    let outcome = common::expect_ok!(delivery).await;
    outcome.accepted_or("not accepted").unwrap();

    sender.close().await.unwrap();
    session_a.close().await.unwrap();
    client_a.close().await.unwrap();
}
