//! Tests sending and receiving small messages with active mq artemis

// TODO: interop testing with other AMQP 1.0 brokers

macro_rules! cfg_not_wasm32 {
    ($($item:item)*) => {
        $(
            #[cfg(not(target_arch = "wasm32"))]
            $item
        )*
    }
}

cfg_not_wasm32! {
    use fe2o3_amqp::{Connection, Receiver, Sender, Session};
    use fe2o3_amqp_types::messaging::Message;

    mod common;

    /// Round trip a payload through the broker and assert byte equality.
    ///
    /// This is modeled on Apache Artemis `AmqpLargeMessageTest`
    /// (`testSendFixedSizedMessages`/`testSend1MBMessage`), which sends
    /// payloads spanning multiple frames (the frame size is at most
    /// 131072 on RabbitMQ and smaller on Artemis/Qpid) and asserts the
    /// received bytes equal the sent bytes.
    async fn send_receive_large_content(url: &str, payload: String, detach_on_close: bool) {
        let mut connection = common::expect_ok!(Connection::open("test-connection", url)).await;
        let mut session = common::expect_ok!(Session::begin(&mut connection)).await;
        let mut sender = common::expect_ok!(Sender::attach(&mut session, "test-sender", "test-queue")).await;
        let mut receiver = common::expect_ok!(Receiver::attach(&mut session, "test-receiver", "test-queue")).await;

        let message = Message::from(payload.clone());
        let outcome = sender.send(message).await.unwrap();
        outcome.accepted_or("Not accepted").unwrap();

        let received = receiver.recv::<String>().await.unwrap();
        receiver.accept(&received).await.unwrap();
        assert_eq!(received.body(), &payload);

        if detach_on_close {
            // rabbitmq only supports non-closing detach
            sender.detach().await.unwrap();
            receiver.detach().await.unwrap();
        } else {
            sender.close().await.unwrap();
            receiver.close().await.unwrap();
        }
        session.close().await.unwrap();
        connection.close().await.unwrap();
    }

    /// Fixed payload sizes that each span multiple frames at the broker's
    /// default frame size (as in Artemis `testSendFixedSizedMessages`).
    const LARGE_CONTENT_SIZES: [usize; 4] = [64 * 1024, 128 * 1024, 256 * 1024, 1024 * 1024];

    #[tokio::test]
    async fn activemq_artemis_send_receive() {
        let (_node, port) = common::setup_activemq_artemis(None, None).await;

        let url = format!("amqp://localhost:{}", port);
        let mut connection = common::expect_ok!(Connection::open("test-connection", &url[..])).await;
        let mut session = common::expect_ok!(Session::begin(&mut connection)).await;
        let mut sender = common::expect_ok!(Sender::attach(&mut session, "test-sender", "test-queue")).await;
        let mut receiver = common::expect_ok!(Receiver::attach(&mut session, "test-receiver", "test-queue")).await;

        let message = Message::from("test-message");
        let outcome = sender.send(message).await.unwrap();
        outcome.accepted_or("Not accepted").unwrap();

        let received = receiver.recv::<String>().await.unwrap();
        receiver.accept(&received).await.unwrap();
        assert_eq!(received.body(), "test-message");

        sender.close().await.unwrap();
        receiver.close().await.unwrap();
        session.close().await.unwrap();
        connection.close().await.unwrap();
    }

    #[tokio::test]
    async fn activemq_artemis_send_receive_large_content() {
        let (_node, port) = common::setup_activemq_artemis(None, None).await;

        let url = format!("amqp://localhost:{}", port);
        for size in LARGE_CONTENT_SIZES {
            send_receive_large_content(&url, "a".repeat(size), false).await;
        }
    }

    #[tokio::test]
    async fn qpid_broker_j_send_receive() {
        let (_node, port) = common::setup_qpid_broker_j(None, None).await;

        let url = format!("amqp://admin:admin@localhost:{}", port);
        let mut connection = common::expect_ok!(Connection::open("test-connection", &url[..])).await;
        let mut session = common::expect_ok!(Session::begin(&mut connection)).await;
        let mut sender = common::expect_ok!(Sender::attach(&mut session, "test-sender", "test-queue")).await;
        let mut receiver = common::expect_ok!(Receiver::attach(&mut session, "test-receiver", "test-queue")).await;

        let message = Message::from("test-message");
        let outcome = sender.send(message).await.unwrap();
        outcome.accepted_or("Not accepted").unwrap();

        let received = receiver.recv::<String>().await.unwrap();
        receiver.accept(&received).await.unwrap();
        assert_eq!(received.body(), "test-message");

        sender.close().await.unwrap();
        receiver.close().await.unwrap();
        session.close().await.unwrap();
        connection.close().await.unwrap();
    }

    #[tokio::test]
    async fn qpid_broker_j_send_receive_large_content() {
        let (_node, port) = common::setup_qpid_broker_j(None, None).await;

        let url = format!("amqp://admin:admin@localhost:{}", port);
        for size in LARGE_CONTENT_SIZES {
            send_receive_large_content(&url, "p".repeat(size), false).await;
        }
    }

    #[tokio::test]
    async fn rabbitmq_amqp10_send_receive() {
        let (_node, port) = common::setup_rabbitmq_amqp10(None, None).await;

        let url = format!("amqp://localhost:{}", port);
        let mut connection = common::expect_ok!(Connection::open("test-connection", &url[..])).await;
        let mut session = common::expect_ok!(Session::begin(&mut connection)).await;
        let mut sender = common::expect_ok!(Sender::attach(&mut session, "test-sender", "test-queue")).await;
        let mut receiver = common::expect_ok!(Receiver::attach(&mut session, "test-receiver", "test-queue")).await;

        let message = Message::from("test-message");
        let outcome = sender.send(message).await.unwrap();
        outcome.accepted_or("Not accepted").unwrap();

        let received = receiver.recv::<String>().await.unwrap();
        receiver.accept(&received).await.unwrap();
        assert_eq!(received.body(), "test-message");

        // rabbitmq only supports non-closing detach
        sender.detach().await.unwrap();
        receiver.detach().await.unwrap();
        session.close().await.unwrap();
        connection.close().await.unwrap();
    }

    #[tokio::test]
    async fn rabbitmq_amqp10_send_receive_large_content() {
        let (_node, port) = common::setup_rabbitmq_amqp10(None, None).await;

        let url = format!("amqp://localhost:{}", port);
        for size in LARGE_CONTENT_SIZES {
            send_receive_large_content(&url, "r".repeat(size), true).await;
        }
    }
}
