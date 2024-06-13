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

    #[tokio::test]
    async fn test_send_receive_compat() {
        activemq_artemis_send_receive().await;
        activemq_artemis_send_receive_large_content().await;
        rabbitmq_amqp10_send_receive().await;
        rabbitmq_amqp10_send_receive_large_content().await;
    }

    async fn activemq_artemis_send_receive() {
        let (_node, port) = common::setup_activemq_artemis(None, None).await;

        let url = format!("amqp://localhost:{}", port);
        let mut connection = Connection::open("test-connection", &url[..]).await.unwrap();
        let mut session = Session::begin(&mut connection).await.unwrap();
        let mut sender = Sender::attach(&mut session, "test-sender", "test-queue")
            .await
            .unwrap();
        let mut receiver = Receiver::attach(&mut session, "test-receiver", "test-queue")
            .await
            .unwrap();

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

    async fn activemq_artemis_send_receive_large_content() {
        let (_node, port) = common::setup_activemq_artemis(None, None).await;

        let url = format!("amqp://localhost:{}", port);
        let mut connection = Connection::open("test-connection", &url[..]).await.unwrap();
        let mut session = Session::begin(&mut connection).await.unwrap();
        let mut sender = Sender::attach(&mut session, "test-sender", "test-queue")
            .await
            .unwrap();
        let mut receiver = Receiver::attach(&mut session, "test-receiver", "test-queue")
            .await
            .unwrap();

        let message = Message::from("test-message".repeat(100_000));
        let outcome = sender.send(message).await.unwrap();
        outcome.accepted_or("Not accepted").unwrap();

        let received = receiver.recv::<String>().await.unwrap();
        receiver.accept(&received).await.unwrap();
        assert_eq!(received.body(), &"test-message".repeat(100_000));

        sender.close().await.unwrap();
        receiver.close().await.unwrap();
        session.close().await.unwrap();
        connection.close().await.unwrap();
    }

    async fn rabbitmq_amqp10_send_receive() {
        let (_node, port) = common::setup_rabbitmq_amqp10(None, None).await;

        let url = format!("amqp://localhost:{}", port);
        let mut connection = Connection::open("test-connection", &url[..]).await.unwrap();
        let mut session = Session::begin(&mut connection).await.unwrap();
        let mut sender = Sender::attach(&mut session, "test-sender", "test-queue")
            .await
            .unwrap();
        let mut receiver = Receiver::attach(&mut session, "test-receiver", "test-queue")
            .await
            .unwrap();

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

    async fn rabbitmq_amqp10_send_receive_large_content() {
        let (_node, port) = common::setup_rabbitmq_amqp10(None, None).await;

        let url = format!("amqp://localhost:{}", port);
        let mut connection = Connection::open("test-connection", &url[..]).await.unwrap();
        let mut session = Session::begin(&mut connection).await.unwrap();
        let mut sender = Sender::attach(&mut session, "test-sender", "test-queue")
            .await
            .unwrap();
        let mut receiver = Receiver::attach(&mut session, "test-receiver", "test-queue")
            .await
            .unwrap();

        let message = Message::from("test-message".repeat(100_000));
        let outcome = sender.send(message).await.unwrap();
        outcome.accepted_or("Not accepted").unwrap();

        let received = receiver.recv::<String>().await.unwrap();
        receiver.accept(&received).await.unwrap();
        assert_eq!(received.body(), &"test-message".repeat(100_000));

        // rabbitmq only supports non-closing detach
        sender.detach().await.unwrap();
        receiver.detach().await.unwrap();
        session.close().await.unwrap();
        connection.close().await.unwrap();
    }
}
