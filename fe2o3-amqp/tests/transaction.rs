//! Tests transactional posting with commit against real brokers

#![cfg(feature = "transaction")]

macro_rules! cfg_not_wasm32 {
    ($($item:item)*) => {
        $(
            #[cfg(not(target_arch = "wasm32"))]
            $item
        )*
    }
}

cfg_not_wasm32! {
    use fe2o3_amqp::{
        connection::Connection,
        session::Session,
        transaction::{Controller, Transaction, TransactionDischarge},
        Receiver, Sender,
    };
    use fe2o3_amqp_types::messaging::Message;

    mod common;

    #[tokio::test]
    async fn test_transactional_posting_commit() {
        qpid_broker_j_post_commit().await;
        activemq_artemis_post_commit().await;
    }

    async fn activemq_artemis_post_commit() {
        let (_node, port) = common::setup_activemq_artemis(None, None).await;
        let url = format!("amqp://localhost:{}", port);
        transactional_posting_commit(&url).await;
    }

    async fn qpid_broker_j_post_commit() {
        let (_node, port) = common::setup_qpid_broker_j(None, None).await;
        let url = format!("amqp://admin:admin@localhost:{}", port);
        transactional_posting_commit(&url).await;
    }

    async fn transactional_posting_commit(url: &str) {
        let mut connection = Connection::open("test-connection", url).await.unwrap();
        let mut session = Session::begin(&mut connection).await.unwrap();
        let controller = Controller::attach(&mut session, "test-controller")
            .await
            .unwrap();
        let mut sender = Sender::attach(&mut session, "test-sender", "test-queue")
            .await
            .unwrap();
        let mut receiver = Receiver::attach(&mut session, "test-receiver", "test-queue")
            .await
            .unwrap();

        let txn = Transaction::declare(&controller, None).await.unwrap();
        let outcome = txn
            .post(&mut sender, Message::from("hello"))
            .await
            .unwrap();
        outcome.accepted_or("Not accepted").unwrap();
        let outcome = txn
            .post(&mut sender, Message::from("world"))
            .await
            .unwrap();
        outcome.accepted_or("Not accepted").unwrap();
        txn.commit().await.unwrap();

        let received = receiver.recv::<String>().await.unwrap();
        receiver.accept(&received).await.unwrap();
        assert_eq!(received.body(), "hello");

        let received = receiver.recv::<String>().await.unwrap();
        receiver.accept(&received).await.unwrap();
        assert_eq!(received.body(), "world");

        controller.close().await.unwrap();
        sender.close().await.unwrap();
        receiver.close().await.unwrap();
        session.close().await.unwrap();
        connection.close().await.unwrap();
    }
}
