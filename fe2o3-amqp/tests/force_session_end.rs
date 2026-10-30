//! Regression test for bounded local session teardown (issue #436).
//!
//! The proxy between the client and the listener withholds the client's `End`
//! for one session, so its graceful end cannot complete. The test then forces
//! that session down, verifies that its engine and links stop, and verifies
//! that a sibling session and the connection remain usable while late frames
//! for the retired channel arrive.

#![cfg(feature = "acceptor")]

use std::time::Duration;

use bytes::BytesMut;
use fe2o3_amqp::{
    acceptor::{
        ConnectionAcceptor, LinkAcceptor, LinkEndpoint, ListenerConnectionHandle,
        ListenerSessionHandle, SessionAcceptor,
    },
    connection::{Connection, ConnectionHandle},
    frames::amqp::{FrameBody, FrameDecoder},
    link::{LinkStateError, RecvError, SessionStopReason},
    session::{Session, SessionHandle},
    types::messaging::{Source, Target},
    Receiver, Sendable, Sender,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::watch,
};
use tokio_util::codec::Decoder;

mod common;

/// A client-side stream proxy that can drop AMQP `End` frames in the
/// client-to-server direction.
async fn establish_filtered_connection_pair() -> (
    ListenerConnectionHandle,
    ConnectionHandle<()>,
    watch::Sender<bool>,
) {
    let (client_io, proxy_client_io) = tokio::io::duplex(64 * 1024);
    let (proxy_server_io, server_io) = tokio::io::duplex(64 * 1024);
    let (drop_end_tx, mut drop_end_rx) = watch::channel(false);

    tokio::spawn(async move {
        let (mut client_read, mut client_write) = tokio::io::split(proxy_client_io);
        let (mut server_read, mut server_write) = tokio::io::split(proxy_server_io);

        // Server -> client is not filtered.
        tokio::spawn(async move {
            let _ = tokio::io::copy(&mut server_read, &mut client_write).await;
        });

        // Forward the client's protocol header verbatim, then relay frames.
        let mut client_header = [0u8; 8];
        if client_read.read_exact(&mut client_header).await.is_err() {
            return;
        }
        if server_write.write_all(&client_header).await.is_err() {
            return;
        }

        loop {
            let mut length = [0u8; 4];
            if client_read.read_exact(&mut length).await.is_err() {
                break;
            }
            let frame_len = u32::from_be_bytes(length) as usize;
            if frame_len < 4 {
                break;
            }
            let mut body = vec![0u8; frame_len - 4];
            if client_read.read_exact(&mut body).await.is_err() {
                break;
            }

            let drop_end = *drop_end_rx.borrow_and_update()
                && matches!(
                    FrameDecoder {}.decode(&mut BytesMut::from(body.as_slice())),
                    Ok(Some(frame)) if matches!(frame.body, FrameBody::End(_))
                );
            if drop_end {
                continue;
            }

            if server_write.write_all(&length).await.is_err()
                || server_write.write_all(&body).await.is_err()
            {
                break;
            }
        }
    });

    let acceptor = ConnectionAcceptor::builder()
        .container_id("force-end-listener")
        .build();
    let connection_task = tokio::spawn(async move { acceptor.accept(server_io).await });

    let client_connection = common::expect_ok!(Connection::builder()
        .container_id("force-end-client")
        .open_with_stream(client_io))
    .await;

    let server_connection = connection_task
        .await
        .expect("connection accept task panicked")
        .expect("connection accept failed");

    (server_connection, client_connection, drop_end_tx)
}

async fn establish_session_pair(
    server_connection: &mut ListenerConnectionHandle,
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

/// Attach a client sender, returning the listener-side receiver.
async fn attach_sender(
    client_session: &mut SessionHandle<()>,
    listener_session: &mut ListenerSessionHandle,
    name: &str,
) -> (Sender, Receiver) {
    let link_acceptor = LinkAcceptor::new();
    let attach_fut = common::expect_ok!(Sender::builder()
        .name(name)
        .source(Source::builder().build())
        .target(Target::builder().build())
        .attach(client_session));
    let (link_result, attach_result) =
        tokio::join!(link_acceptor.accept(listener_session), attach_fut);
    let server_receiver = match link_result.expect("link accept failed") {
        LinkEndpoint::Receiver(receiver) => receiver,
        other => panic!("expected receiver endpoint, got {other:?}"),
    };
    (attach_result, server_receiver)
}

/// Attach a client receiver, returning the listener-side sender.
async fn attach_receiver(
    client_session: &mut SessionHandle<()>,
    listener_session: &mut ListenerSessionHandle,
    name: &str,
) -> (Receiver, Sender) {
    let link_acceptor = LinkAcceptor::new();
    let attach_fut = common::expect_ok!(Receiver::builder()
        .name(name)
        .source(Source::builder().build())
        .target(Target::builder().build())
        .attach(client_session));
    let (link_result, attach_result) =
        tokio::join!(link_acceptor.accept(listener_session), attach_fut);
    let server_sender = match link_result.expect("link accept failed") {
        LinkEndpoint::Sender(sender) => sender,
        other => panic!("expected sender endpoint, got {other:?}"),
    };
    (attach_result, server_sender)
}

async fn send_and_receive(sender: &mut Sender, receiver: &mut Receiver, body: &str) {
    common::expect_ok!(sender.send(Sendable::builder().message(body).settled(true).build())).await;
    let delivery = common::expect_ok!(receiver.recv::<String>()).await;
    assert_eq!(delivery.body(), body);
    common::expect_ok!(receiver.accept(&delivery)).await;
}

#[tokio::test]
async fn force_end_is_bounded_and_preserves_sibling_sessions() {
    let (mut server_connection, mut client_connection, drop_end) =
        establish_filtered_connection_pair().await;

    let (mut client_session_1, mut server_session_1) =
        establish_session_pair(&mut server_connection, &mut client_connection).await;
    let (mut client_session_2, mut server_session_2) =
        establish_session_pair(&mut server_connection, &mut client_connection).await;

    // Session 1 has a listener-side sender so it can produce late frames for
    // the retired channel after the client has forced the session down.
    let (mut late_receiver, mut late_sender) =
        attach_receiver(&mut client_session_1, &mut server_session_1, "late-recv").await;
    // Session 2 is the sibling that must keep working.
    let (mut sibling_sender, mut sibling_receiver) =
        attach_sender(&mut client_session_2, &mut server_session_2, "sibling").await;

    send_and_receive(&mut sibling_sender, &mut sibling_receiver, "before").await;

    // Withhold the client's End for session 1, so graceful end times out.
    drop_end.send(true).unwrap();
    let graceful = tokio::time::timeout(Duration::from_millis(200), client_session_1.end()).await;
    assert!(
        graceful.is_err(),
        "graceful end must not complete while the peer withholds End, got {graceful:?}"
    );

    // Local teardown must complete without a peer response.
    common::expect_ok!(client_session_1.force_end()).await;
    assert!(
        client_session_1.is_ended(),
        "the session engine must have stopped"
    );
    // The link waiters observe the session stop.
    match tokio::time::timeout(Duration::from_secs(2), late_receiver.recv::<String>()).await {
        Ok(Err(RecvError::LinkStateError(LinkStateError::SessionStopped(reason)))) => {
            assert_eq!(reason, SessionStopReason::Ended);
        }
        other => panic!("expected the link to wake with the session stop, got {other:?}"),
    }

    // A late transfer on the retired channel must be discarded without
    // taking down the connection.
    let _ = late_sender
        .send(Sendable::builder().message("late").settled(true).build())
        .await;

    // The sibling session still works.
    send_and_receive(&mut sibling_sender, &mut sibling_receiver, "after").await;

    // A new session can still be begun. Its channel must not collide with the
    // retired one (the connection keeps the slot reserved).
    drop_end.send(false).unwrap();
    let (mut client_session_3, _server_session_3) =
        establish_session_pair(&mut server_connection, &mut client_connection).await;
    common::expect_ok!(client_session_3.end()).await;

    assert!(
        !client_connection.is_closed(),
        "the connection must remain open"
    );
}

/// Forcing a session down without a preceding graceful End reports
/// `SessionStopReason::Aborted` to links and leaves sibling sessions usable.
#[tokio::test]
async fn force_end_without_graceful_end_reports_aborted() {
    let (mut server_connection, mut client_connection, _drop_end) =
        establish_filtered_connection_pair().await;

    let (mut client_session_1, mut server_session_1) =
        establish_session_pair(&mut server_connection, &mut client_connection).await;
    let (mut client_session_2, mut server_session_2) =
        establish_session_pair(&mut server_connection, &mut client_connection).await;

    let (mut late_receiver, _late_sender) =
        attach_receiver(&mut client_session_1, &mut server_session_1, "abort-recv").await;
    let (mut sibling_sender, mut sibling_receiver) =
        attach_sender(&mut client_session_2, &mut server_session_2, "sibling").await;

    common::expect_ok!(client_session_1.force_end()).await;

    match tokio::time::timeout(Duration::from_secs(2), late_receiver.recv::<String>()).await {
        Ok(Err(RecvError::LinkStateError(LinkStateError::SessionStopped(reason)))) => {
            assert_eq!(reason, SessionStopReason::Aborted);
        }
        other => panic!("expected the link to wake with Aborted, got {other:?}"),
    }

    send_and_receive(&mut sibling_sender, &mut sibling_receiver, "after-abort").await;
    assert!(!client_connection.is_closed());
}
