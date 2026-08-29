//! Tests that large messages are split across multiple transfer frames
//! bounded by the negotiated max frame size (not the max message size).
//!
//! These run over an in-memory `tokio::io::duplex` stream, so no broker or
//! network is required. The scenarios are modeled on how brokers exercise
//! multi-frame deliveries themselves (e.g. Apache Artemis
//! `AmqpLargeMessageTest`, which parameterizes the frame size against
//! payload sizes spanning multiple frames and verifies byte-identical
//! round trips).
//!
//! Note: unlike real brokers, the fe2o3 acceptor receiver does not
//! auto-accept at ingress, so the sender's `send()` only completes after
//! the receiver accepts the delivery. The tests therefore receive/accept
//! concurrently with sending.

#![cfg(feature = "acceptor")]

use std::time::Duration;

use fe2o3_amqp::{
    acceptor::{
        ConnectionAcceptor, LinkAcceptor, LinkEndpoint, ListenerSessionHandle, SessionAcceptor,
    },
    connection::{Connection, ConnectionHandle},
    link::SendError,
    session::{Session, SessionHandle},
    types::messaging::Message,
    Sender,
};

const FRAME_SIZE: usize = 1024;

async fn establish_connection_pair(
    frame_size: usize,
) -> (
    fe2o3_amqp::acceptor::ListenerConnectionHandle,
    ConnectionHandle<()>,
) {
    let (client_io, server_io) = tokio::io::duplex(64 * 1024);

    let acceptor = ConnectionAcceptor::builder()
        .container_id("test-listener")
        .max_frame_size(frame_size as u32)
        .build();
    let connection_task = tokio::spawn(async move { acceptor.accept(server_io).await });

    let client_connection = Connection::builder()
        .container_id("test-client")
        .max_frame_size(frame_size as u32)
        .open_with_stream(client_io)
        .await
        .expect("client connection failed");

    let server_connection = connection_task
        .await
        .expect("connection accept task panicked")
        .expect("connection accept failed");

    (server_connection, client_connection)
}

async fn establish_session_pair(
    server_connection: &mut fe2o3_amqp::acceptor::ListenerConnectionHandle,
    client_connection: &mut ConnectionHandle<()>,
    session_incoming_window: u32,
) -> (ListenerSessionHandle, SessionHandle<()>) {
    let session_acceptor = SessionAcceptor::builder()
        .incoming_window(session_incoming_window)
        .build();
    let (session_result, begin_result) = tokio::join!(
        session_acceptor.accept(server_connection),
        Session::builder()
            .incoming_window(session_incoming_window)
            .begin(client_connection),
    );
    let client_session = begin_result.expect("client session begin failed");
    let listener_session = session_result.expect("session accept failed");
    (listener_session, client_session)
}

/// Attach the client sender concurrently with the acceptor accepting the
/// incoming link, returning both ends.
async fn establish_link_pair(
    listener_session: &mut ListenerSessionHandle,
    client_session: &mut SessionHandle<()>,
) -> (Sender, fe2o3_amqp::Receiver) {
    let link_acceptor = LinkAcceptor::builder().build();
    let (accept_result, attach_result) = tokio::join!(
        link_acceptor.accept(listener_session),
        Sender::attach(client_session, "test-sender", "test-queue"),
    );

    let sender = attach_result.expect("sender attach failed");
    let receiver = match accept_result.expect("link accept failed") {
        LinkEndpoint::Receiver(receiver) => receiver,
        other => panic!("expected receiver endpoint, got {:?}", other),
    };
    (sender, receiver)
}

/// A payload larger than a single frame must be split across multiple
/// transfer frames at the frame-size boundary, not the max-message-size
/// boundary, and must round trip byte-identically. Payload sizes straddle
/// the frame boundary (frame * 2, frame * 2 + 1, frame * 4, ...) as in
/// Artemis `AmqpLargeMessageTest::testSendFixedSizedMessages`.
#[tokio::test]
async fn payloads_larger_than_frame_size_round_trip() {
    let (mut server_connection, mut client_connection) =
        establish_connection_pair(FRAME_SIZE).await;
    let (mut listener_session, mut client_session) =
        establish_session_pair(&mut server_connection, &mut client_connection, 5000).await;

    for size in [
        2 * FRAME_SIZE,
        2 * FRAME_SIZE + 1,
        4 * FRAME_SIZE,
        8 * FRAME_SIZE + 17,
        128 * FRAME_SIZE,
    ] {
        let (mut sender, mut receiver) =
            establish_link_pair(&mut listener_session, &mut client_session).await;

        let payload = "x".repeat(size);
        let send_payload = payload.clone();
        let send_task = tokio::spawn(async move {
            let message = Message::from(send_payload);
            let outcome = sender.send(message).await.unwrap();
            outcome.accepted_or("Not accepted").unwrap();
            sender.close().await.unwrap();
        });

        let received = tokio::time::timeout(Duration::from_secs(10), receiver.recv::<String>())
            .await
            .expect("timed out waiting for message")
            .expect("recv failed");
        receiver.accept(&received).await.unwrap();
        assert_eq!(
            received.body(),
            &payload,
            "payload of size {size} round trip failed"
        );
        receiver.close().await.unwrap();
        send_task.await.unwrap();
    }

    client_session.close().await.unwrap();
    client_connection.close().await.unwrap();
}

/// Many multi-frame messages sent back to back with a small session
/// incoming-window exercise the session window re-advertisement and the
/// per-transfer accounting: every physical transfer frame must be counted
/// in the session's `next-outgoing-id` / `remote-incoming-window`.
#[tokio::test]
async fn many_multiframe_messages_with_small_window() {
    const MESSAGE_COUNT: usize = 30;
    const PAYLOAD_SIZE: usize = 4 * FRAME_SIZE;

    let (mut server_connection, mut client_connection) =
        establish_connection_pair(FRAME_SIZE).await;
    let (mut listener_session, mut client_session) =
        establish_session_pair(&mut server_connection, &mut client_connection, 8).await;

    let (mut sender, mut receiver) =
        establish_link_pair(&mut listener_session, &mut client_session).await;

    let payload = "y".repeat(PAYLOAD_SIZE);
    let send_payload = payload.clone();
    let send_task = tokio::spawn(async move {
        for _ in 0..MESSAGE_COUNT {
            let message = Message::from(send_payload.clone());
            let outcome = sender.send(message).await.unwrap();
            outcome.accepted_or("Not accepted").unwrap();
        }
        sender.close().await.unwrap();
    });

    for _ in 0..MESSAGE_COUNT {
        let received = tokio::time::timeout(Duration::from_secs(10), receiver.recv::<String>())
            .await
            .expect("timed out waiting for message")
            .expect("recv failed");
        receiver.accept(&received).await.unwrap();
        assert_eq!(received.body(), &payload);
    }
    receiver.close().await.unwrap();
    send_task.await.unwrap();

    client_session.close().await.unwrap();
    client_connection.close().await.unwrap();
}

/// A message larger than the negotiated max-message-size of the link must be
/// rejected locally with `SendError::MessageSizeExceeded` before any transfer
/// frame is sent (mirroring go-amqp's `amqp:link:message-size-exceeded`
/// behavior), rather than being split across frames.
#[tokio::test]
async fn message_larger_than_max_message_size_is_rejected() {
    let (mut server_connection, mut client_connection) =
        establish_connection_pair(FRAME_SIZE).await;
    let (mut listener_session, mut client_session) =
        establish_session_pair(&mut server_connection, &mut client_connection, 5000).await;

    let link_acceptor = LinkAcceptor::builder().max_message_size(1000u64).build();
    let (accept_result, attach_result) = tokio::join!(
        link_acceptor.accept(&mut listener_session),
        Sender::attach(&mut client_session, "test-sender", "test-queue"),
    );
    let mut sender = attach_result.expect("sender attach failed");
    let receiver = match accept_result.expect("link accept failed") {
        LinkEndpoint::Receiver(receiver) => receiver,
        other => panic!("expected receiver endpoint, got {:?}", other),
    };

    let message = Message::from("z".repeat(2000));
    let error = sender.send(message).await.expect_err("expected error");
    match error {
        SendError::MessageSizeExceeded(e) => {
            assert_eq!(e.max_size, 1000);
            assert!(e.size >= 2000, "encoded size was {}", e.size);
        }
        other => panic!("expected MessageSizeExceeded, got {:?}", other),
    }

    // The sender and receiver closes must run concurrently: each side
    // processes the other's closing detach as part of its own close handshake.
    let (sender_close, receiver_close) = tokio::join!(sender.close(), receiver.close());
    sender_close.unwrap();
    receiver_close.unwrap();
    client_session.close().await.unwrap();
    client_connection.close().await.unwrap();
}

/// Purpose of this test:
///
/// 1. **End-to-end coverage of cross-connection resume.** Resuming a link on
///    a session of a *different connection* (`detach()` + `resume_on_session`)
///    exercises the full path: detach handshake, session/outgoing switch,
///    reattach exchange, and subsequent sends. This path is otherwise
///    untested.
/// 2. **Behavioral guard for the max-frame-size refresh.** The link's
///    `max_frame_size` must be refreshed from the new session when the link
///    moves to a connection with a different negotiated frame size. Here the
///    sender moves from a connection with 4096-byte frames to one with
///    1024-byte frames; the ~3000-byte message can therefore only be sent as
///    a multi-frame delivery if the refreshed value is in effect. A stale
///    value would silently fall back to the transport's defensive frame
///    splitting — the bug class this work eliminates. The split boundary
///    itself is not publicly observable (there is no public max-frame-size
///    getter), so this is behavioral; the field refresh itself is covered by
///    the `switch_session` unit tests.
/// 3. **Post-resume usability.** A second send on the resumed link confirms
///    that credit, flow, and settlement keep working after the move.
#[tokio::test]
async fn resume_on_new_connection_refreshes_max_frame_size() {
    // Two connection pairs with different negotiated frame sizes
    let (mut server_a, mut client_a) = establish_connection_pair(4096).await;
    let (mut server_b, mut client_b) = establish_connection_pair(1024).await;
    let (mut listener_a, mut session_a) =
        establish_session_pair(&mut server_a, &mut client_a, 5000).await;
    let (mut listener_b, mut session_b) =
        establish_session_pair(&mut server_b, &mut client_b, 5000).await;

    // Attach the sender on connection A
    let link_acceptor_a = LinkAcceptor::builder().build();
    let (accept_a, attach_a) = tokio::join!(
        link_acceptor_a.accept(&mut listener_a),
        Sender::attach(&mut session_a, "test-sender", "test-queue"),
    );
    let sender = attach_a.expect("sender attach failed");
    let mut receiver_a = match accept_a.expect("link accept failed") {
        LinkEndpoint::Receiver(receiver) => receiver,
        other => panic!("expected receiver endpoint, got {:?}", other),
    };

    // Drive the acceptor-side receiver so it responds to the sender's
    // non-closing detach (the receiver's recv loop performs the detach
    // handshake and then errors with `RemoteDetached`).
    let recv_a_task = tokio::spawn(async move {
        let _ = receiver_a.recv::<String>().await;
    });

    // Accept the resumed link on connection B FIRST: the accept task must run
    // concurrently with the resume's attach exchange, which waits for the
    // server's attach reply (keeping the listener session alive by returning
    // it from the task).
    let link_acceptor_b = LinkAcceptor::builder().build();
    let accept_b_task = tokio::spawn(async move {
        let link = link_acceptor_b.accept(&mut listener_b).await.unwrap();
        (link, listener_b)
    });

    // Detach on A and resume the sender on connection B's session
    let detached = sender.detach().await.unwrap();
    recv_a_task.await.unwrap();
    let mut sender = detached.resume_on_session(&session_b).await.unwrap();

    // Keep the listener session alive for the rest of the test (dropping it
    // would end the server-side session)
    let (endpoint_b, _listener_b) = accept_b_task.await.unwrap();
    let mut receiver_b = match endpoint_b {
        LinkEndpoint::Receiver(receiver) => receiver,
        other => panic!("expected receiver endpoint, got {:?}", other),
    };

    // Send the two messages from a spawned task (the send blocks until the
    // receiver accepts), receiving each on B's acceptor concurrently
    let payload = "x".repeat(3000);
    let small_payload = "small".to_string();
    let send_payload = payload.clone();
    let send_small_payload = small_payload.clone();
    let send_task = tokio::spawn(async move {
        let message = Message::from(send_payload);
        let outcome = sender.send(message).await.unwrap();
        outcome.accepted_or("Not accepted").unwrap();

        // The resumed link stays usable: a second, smaller send
        let message = Message::from(send_small_payload);
        let outcome = sender.send(message).await.unwrap();
        outcome.accepted_or("Not accepted").unwrap();

        sender.close().await.unwrap();
    });

    let received = receiver_b.recv::<String>().await.unwrap();
    receiver_b.accept(&received).await.unwrap();
    assert_eq!(received.body(), &payload);

    let received = receiver_b.recv::<String>().await.unwrap();
    receiver_b.accept(&received).await.unwrap();
    assert_eq!(received.body(), &small_payload);

    receiver_b.close().await.unwrap();
    send_task.await.unwrap();

    session_b.close().await.unwrap();
    client_b.close().await.unwrap();
    client_a.close().await.unwrap();
}
