//! Tests transactional support against real brokers:
//! - posting (all four variants) with commit/rollback
//! - retirement (accept/reject/release/modify/retire)
//! - acquisition (Qpid only; Artemis does not implement it)
//!
//! Both `Transaction` (shared control link) and `OwnedTransaction` (owned control
//! link) are exercised, each with their own connection/session/controller.

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
    use std::fmt::Debug;
    use std::time::Duration;

    use fe2o3_amqp::{
        connection::Connection,
        link::FlowError,
        session::{Session, SessionHandle},
        transaction::{
            Controller, OwnedTransaction, Transaction, TransactionAcquisition,
            TransactionDischarge, TransactionPosting, TransactionRetirement,
        },
        Delivery, Receiver, Sendable, Sender,
    };
    use fe2o3_amqp_types::messaging::{Message, Modified, Outcome, Released};
    use tokio::time::timeout;

    mod common;

    #[tokio::test]
    async fn test_qpid_transactions() {
        let (_node, port) = common::setup_qpid_broker_j(None, None).await;
        let url = format!("amqp://admin:admin@localhost:{}", port);
        transaction_posting_commit(&url).await;
        owned_transaction_posting_commit(&url).await;
        transaction_discharge(&url).await;
        owned_transaction_discharge(&url).await;
        posting_variants(&url).await;
        owned_posting_variants(&url).await;
        transaction_rollback(&url).await;
        owned_transaction_rollback(&url).await;
        qpid_retirement(&url).await;
        qpid_owned_retirement(&url).await;
        acquisition(&url).await;
        owned_acquisition(&url).await;
    }

    #[tokio::test]
    async fn test_artemis_transactions() {
        let (_node, port) = common::setup_activemq_artemis(None, None).await;
        let url = format!("amqp://localhost:{}", port);
        transaction_posting_commit(&url).await;
        owned_transaction_posting_commit(&url).await;
        transaction_discharge(&url).await;
        owned_transaction_discharge(&url).await;
        posting_variants(&url).await;
        owned_posting_variants(&url).await;
        transaction_rollback(&url).await;
        owned_transaction_rollback(&url).await;
        artemis_retirement(&url).await;
        artemis_owned_retirement(&url).await;
    }

    // ----- posting + commit -----

    async fn post_commit_and_verify<T>(
        txn: T,
        sender: &mut Sender,
        receiver: &mut Receiver,
    ) where
        T: TransactionPosting + TransactionDischarge + Send + Sync,
        <T as TransactionDischarge>::Error: Debug,
    {
        assert!(!txn.is_discharged());
        let outcome = txn.post(sender, Message::from("hello")).await.unwrap();
        outcome.accepted_or("Not accepted").unwrap();
        let outcome = txn.post(sender, Message::from("world")).await.unwrap();
        outcome.accepted_or("Not accepted").unwrap();
        txn.commit().await.unwrap();

        let received = receiver.recv::<String>().await.unwrap();
        receiver.accept(&received).await.unwrap();
        assert_eq!(received.body(), "hello");

        let received = receiver.recv::<String>().await.unwrap();
        receiver.accept(&received).await.unwrap();
        assert_eq!(received.body(), "world");
    }

    async fn transaction_posting_commit(url: &str) {
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
        post_commit_and_verify(txn, &mut sender, &mut receiver).await;

        controller.close().await.unwrap();
        sender.close().await.unwrap();
        receiver.close().await.unwrap();
        session.close().await.unwrap();
        connection.close().await.unwrap();
    }

    async fn owned_transaction_posting_commit(url: &str) {
        let mut connection = Connection::open("test-connection", url).await.unwrap();
        let mut session = Session::begin(&mut connection).await.unwrap();
        let mut sender = Sender::attach(&mut session, "test-sender", "test-queue")
            .await
            .unwrap();
        let mut receiver = Receiver::attach(&mut session, "test-receiver", "test-queue")
            .await
            .unwrap();

        let txn = OwnedTransaction::declare(&mut session, "test-owned-controller", None)
            .await
            .unwrap();
        post_commit_and_verify(txn, &mut sender, &mut receiver).await;

        sender.close().await.unwrap();
        receiver.close().await.unwrap();
        session.close().await.unwrap();
        connection.close().await.unwrap();
    }

    // ----- discharge + is_discharged -----

    async fn discharge_and_verify<T>(
        mut txn: T,
        sender: &mut Sender,
        receiver: &mut Receiver,
    ) where
        T: TransactionPosting + TransactionDischarge + Send + Sync,
        <T as TransactionDischarge>::Error: Debug,
    {
        assert!(!txn.is_discharged());
        let outcome = txn.post(sender, Message::from("discharge")).await.unwrap();
        outcome.accepted_or("Not accepted").unwrap();
        txn.discharge(false).await.unwrap();
        assert!(txn.is_discharged());

        let received = receiver.recv::<String>().await.unwrap();
        receiver.accept(&received).await.unwrap();
        assert_eq!(received.body(), "discharge");
    }

    async fn transaction_discharge(url: &str) {
        let mut connection = Connection::open("test-connection", url).await.unwrap();
        let mut session = Session::begin(&mut connection).await.unwrap();
        let controller = Controller::attach(&mut session, "test-controller")
            .await
            .unwrap();
        let mut sender = Sender::attach(&mut session, "test-sender", "test-queue-discharge")
            .await
            .unwrap();
        let mut receiver =
            Receiver::attach(&mut session, "test-receiver", "test-queue-discharge")
                .await
                .unwrap();

        let txn = Transaction::declare(&controller, None).await.unwrap();
        discharge_and_verify(txn, &mut sender, &mut receiver).await;

        controller.close().await.unwrap();
        sender.close().await.unwrap();
        receiver.close().await.unwrap();
        session.close().await.unwrap();
        connection.close().await.unwrap();
    }

    async fn owned_transaction_discharge(url: &str) {
        let mut connection = Connection::open("test-connection", url).await.unwrap();
        let mut session = Session::begin(&mut connection).await.unwrap();
        let mut sender = Sender::attach(&mut session, "test-sender", "test-queue-discharge")
            .await
            .unwrap();
        let mut receiver =
            Receiver::attach(&mut session, "test-receiver", "test-queue-discharge")
                .await
                .unwrap();

        let txn = OwnedTransaction::declare(&mut session, "test-owned-controller", None)
            .await
            .unwrap();
        discharge_and_verify(txn, &mut sender, &mut receiver).await;

        sender.close().await.unwrap();
        receiver.close().await.unwrap();
        session.close().await.unwrap();
        connection.close().await.unwrap();
    }

    // ----- posting variants -----

    async fn post_variants_and_verify<T>(
        txn: T,
        sender: &mut Sender,
        receiver: &mut Receiver,
    ) where
        T: TransactionPosting + TransactionDischarge + Send + Sync,
        <T as TransactionDischarge>::Error: Debug,
    {
        let outcome = txn.post(sender, Message::from("post")).await.unwrap();
        outcome.accepted_or("Not accepted").unwrap();

        let sendable = Sendable::builder()
            .message(Message::from("post_ref"))
            .build();
        let outcome = txn.post_ref(sender, &sendable).await.unwrap();
        outcome.accepted_or("Not accepted").unwrap();

        let fut = txn
            .post_batchable(sender, Message::from("post_batchable"))
            .await
            .unwrap();
        let outcome = fut.await.unwrap();
        outcome.accepted_or("Not accepted").unwrap();

        let sendable = Sendable::builder()
            .message(Message::from("post_batchable_ref"))
            .build();
        let fut = txn.post_batchable_ref(sender, &sendable).await.unwrap();
        let outcome = fut.await.unwrap();
        outcome.accepted_or("Not accepted").unwrap();

        txn.commit().await.unwrap();

        for expected in ["post", "post_ref", "post_batchable", "post_batchable_ref"] {
            let received = receiver.recv::<String>().await.unwrap();
            receiver.accept(&received).await.unwrap();
            assert_eq!(received.body(), expected);
        }
    }

    async fn posting_variants(url: &str) {
        let mut connection = Connection::open("test-connection", url).await.unwrap();
        let mut session = Session::begin(&mut connection).await.unwrap();
        let controller = Controller::attach(&mut session, "test-controller")
            .await
            .unwrap();
        let mut sender =
            Sender::attach(&mut session, "test-sender", "test-queue-variants")
                .await
                .unwrap();
        let mut receiver = Receiver::attach(
            &mut session,
            "test-receiver",
            "test-queue-variants",
        )
        .await
        .unwrap();

        let txn = Transaction::declare(&controller, None).await.unwrap();
        post_variants_and_verify(txn, &mut sender, &mut receiver).await;

        controller.close().await.unwrap();
        sender.close().await.unwrap();
        receiver.close().await.unwrap();
        session.close().await.unwrap();
        connection.close().await.unwrap();
    }

    async fn owned_posting_variants(url: &str) {
        let mut connection = Connection::open("test-connection", url).await.unwrap();
        let mut session = Session::begin(&mut connection).await.unwrap();
        let mut sender =
            Sender::attach(&mut session, "test-sender", "test-queue-variants")
                .await
                .unwrap();
        let mut receiver = Receiver::attach(
            &mut session,
            "test-receiver",
            "test-queue-variants",
        )
        .await
        .unwrap();

        let txn = OwnedTransaction::declare(&mut session, "test-owned-controller", None)
            .await
            .unwrap();
        post_variants_and_verify(txn, &mut sender, &mut receiver).await;

        sender.close().await.unwrap();
        receiver.close().await.unwrap();
        session.close().await.unwrap();
        connection.close().await.unwrap();
    }

    // ----- rollback -----

    async fn rollback_and_verify<T>(
        txn: T,
        sender: &mut Sender,
        receiver: &mut Receiver,
    ) where
        T: TransactionPosting + TransactionDischarge + Send + Sync,
        <T as TransactionDischarge>::Error: Debug,
    {
        for body in ["a", "b"] {
            let outcome = txn.post(sender, Message::from(body)).await.unwrap();
            outcome.accepted_or("Not accepted").unwrap();
        }
        txn.rollback().await.unwrap();

        let result = timeout(Duration::from_secs(3), receiver.recv::<String>()).await;
        assert!(result.is_err(), "message arrived after rollback");
    }

    async fn transaction_rollback(url: &str) {
        let mut connection = Connection::open("test-connection", url).await.unwrap();
        let mut session = Session::begin(&mut connection).await.unwrap();
        let controller = Controller::attach(&mut session, "test-controller")
            .await
            .unwrap();
        let mut sender = Sender::attach(&mut session, "test-sender", "test-queue-rollback")
            .await
            .unwrap();
        let mut receiver =
            Receiver::attach(&mut session, "test-receiver", "test-queue-rollback")
                .await
                .unwrap();

        let txn = Transaction::declare(&controller, None).await.unwrap();
        rollback_and_verify(txn, &mut sender, &mut receiver).await;

        // A subsequent transaction on the same controller still works
        let txn = Transaction::declare(&controller, None).await.unwrap();
        post_commit_and_verify(txn, &mut sender, &mut receiver).await;

        controller.close().await.unwrap();
        sender.close().await.unwrap();
        receiver.close().await.unwrap();
        session.close().await.unwrap();
        connection.close().await.unwrap();
    }

    async fn owned_transaction_rollback(url: &str) {
        let mut connection = Connection::open("test-connection", url).await.unwrap();
        let mut session = Session::begin(&mut connection).await.unwrap();
        let mut sender = Sender::attach(&mut session, "test-sender", "test-queue-rollback")
            .await
            .unwrap();
        let mut receiver =
            Receiver::attach(&mut session, "test-receiver", "test-queue-rollback")
                .await
                .unwrap();

        let txn = OwnedTransaction::declare(&mut session, "test-owned-controller-0", None)
            .await
            .unwrap();
        rollback_and_verify(txn, &mut sender, &mut receiver).await;

        // A subsequent transaction on its own control link still works
        let txn = OwnedTransaction::declare(&mut session, "test-owned-controller-1", None)
            .await
            .unwrap();
        post_commit_and_verify(txn, &mut sender, &mut receiver).await;

        sender.close().await.unwrap();
        receiver.close().await.unwrap();
        session.close().await.unwrap();
        connection.close().await.unwrap();
    }

    // ----- retirement -----

    /// The message bodies of the retirement sub-cases. Each case also uses a queue
    /// named `test-queue-ret-<body>`.
    const RETIREMENT_BODIES: [&str; 6] = [
        "rollback-accept",
        "accept",
        "reject",
        "release",
        "modify",
        "retire",
    ];

    async fn recv_string(receiver: &mut Receiver) -> Delivery<String> {
        timeout(Duration::from_secs(5), receiver.recv::<String>())
            .await
            .expect("timed out waiting for a delivery")
            .unwrap()
    }

    async fn expect_redelivered(receiver: &mut Receiver, expected: &str) {
        let redelivered = recv_string(receiver).await;
        assert_eq!(redelivered.body(), expected);
        receiver.accept(&redelivered).await.unwrap();
    }

    async fn retire_accept_rollback<T>(txn: T, receiver: &mut Receiver, expected: &str)
    where
        T: TransactionRetirement + TransactionDischarge + Send + Sync,
        T::RetireError: Debug,
        <T as TransactionDischarge>::Error: Debug,
    {
        let delivery = recv_string(receiver).await;
        assert_eq!(delivery.body(), expected);
        txn.accept(receiver, &delivery).await.unwrap();
        txn.rollback().await.unwrap();
    }

    async fn retire_accept_commit<T>(txn: T, receiver: &mut Receiver, expected: &str)
    where
        T: TransactionRetirement + TransactionDischarge + Send + Sync,
        T::RetireError: Debug,
        <T as TransactionDischarge>::Error: Debug,
    {
        let delivery = recv_string(receiver).await;
        assert_eq!(delivery.body(), expected);
        txn.accept(receiver, &delivery).await.unwrap();
        txn.commit().await.unwrap();
    }

    /// Retire a delivery with a `Rejected` outcome and commit.
    ///
    /// The message's fate after the commit depends on the broker, so the caller
    /// decides whether to verify the redelivery (the redelivery is verified by the caller).
    async fn retire_reject_commit<T>(txn: T, receiver: &mut Receiver, expected: &str)
    where
        T: TransactionRetirement + TransactionDischarge + Send + Sync,
        T::RetireError: Debug,
        <T as TransactionDischarge>::Error: Debug,
    {
        let delivery = recv_string(receiver).await;
        assert_eq!(delivery.body(), expected);
        txn.reject(receiver, &delivery, None).await.unwrap();
        txn.commit().await.unwrap();
    }

    /// Retire a delivery with a `Released` outcome and commit.
    ///
    /// The message's fate after the commit depends on the broker, so the caller
    /// decides whether to verify the redelivery (the redelivery is verified by the caller).
    async fn retire_release_commit<T>(txn: T, receiver: &mut Receiver, expected: &str)
    where
        T: TransactionRetirement + TransactionDischarge + Send + Sync,
        T::RetireError: Debug,
        <T as TransactionDischarge>::Error: Debug,
    {
        let delivery = recv_string(receiver).await;
        assert_eq!(delivery.body(), expected);
        txn.release(receiver, &delivery).await.unwrap();
        txn.commit().await.unwrap();
    }

    /// Retire a delivery with a `Modified` outcome and commit.
    ///
    /// The message's fate after the commit depends on the broker, so the caller
    /// decides whether to verify the redelivery (the redelivery is verified by the caller).
    async fn retire_modify_commit<T>(txn: T, receiver: &mut Receiver, expected: &str)
    where
        T: TransactionRetirement + TransactionDischarge + Send + Sync,
        T::RetireError: Debug,
        <T as TransactionDischarge>::Error: Debug,
    {
        let delivery = recv_string(receiver).await;
        assert_eq!(delivery.body(), expected);
        let modified = Modified {
            delivery_failed: None,
            undeliverable_here: None,
            message_annotations: None,
        };
        txn.modify(receiver, &delivery, modified).await.unwrap();
        txn.commit().await.unwrap();
    }

    /// Retire a delivery with an explicit outcome and commit.
    ///
    /// The message's fate after the commit depends on the broker, so the caller
    /// decides whether to verify the redelivery.
    async fn retire_outcome_commit<T>(txn: T, receiver: &mut Receiver, expected: &str)
    where
        T: TransactionRetirement + TransactionDischarge + Send + Sync,
        T::RetireError: Debug,
        <T as TransactionDischarge>::Error: Debug,
    {
        let delivery = recv_string(receiver).await;
        assert_eq!(delivery.body(), expected);
        txn.retire(receiver, &delivery, Outcome::Released(Released {}))
            .await
            .unwrap();
        txn.commit().await.unwrap();
    }

    /// Run one Qpid retirement sub-case: attach the seed sender and receiver for
    /// the case's queue, seed the message, and exercise the retirement method
    /// selected by `body`.
    ///
    /// Qpid applies every transactional outcome: an `Accepted` retirement consumes
    /// the message, while `Rejected`, `Released`, and `Modified` ones return it to
    /// the queue (rejected messages are released with an incremented delivery
    /// count). The returned messages are verified via [`expect_redelivered`].
    ///
    /// Each sub-case uses its own queue and receiver so that a late redelivery of an
    /// earlier case cannot interfere with the next one. The receiver is attached
    /// before seeding: on Artemis a message that is enqueued before the consuming
    /// link attaches is not delivered.
    async fn qpid_retire_case<T, R>(
        body: &str,
        txn: T,
        session: &mut SessionHandle<R>,
    ) where
        T: TransactionRetirement + TransactionDischarge + Send + Sync,
        T::RetireError: Debug,
        <T as TransactionDischarge>::Error: Debug,
    {
        let queue = format!("test-queue-ret-{}", body);

        let mut seed_sender = Sender::attach(
            session,
            format!("test-seed-sender-{}", body),
            &queue,
        )
        .await
        .unwrap();
        let mut receiver = Receiver::attach(
            session,
            format!("test-receiver-{}", body),
            &queue,
        )
        .await
        .unwrap();
        seed_sender.send(Message::from(body)).await.unwrap();

        match body {
            "rollback-accept" => {
                retire_accept_rollback(txn, &mut receiver, body).await;
                expect_redelivered(&mut receiver, body).await;
            }
            "accept" => retire_accept_commit(txn, &mut receiver, body).await,
            "reject" => {
                retire_reject_commit(txn, &mut receiver, body).await;
                expect_redelivered(&mut receiver, body).await;
            }
            "release" => {
                retire_release_commit(txn, &mut receiver, body).await;
                expect_redelivered(&mut receiver, body).await;
            }
            "modify" => {
                retire_modify_commit(txn, &mut receiver, body).await;
                expect_redelivered(&mut receiver, body).await;
            }
            _ => {
                retire_outcome_commit(txn, &mut receiver, body).await;
                expect_redelivered(&mut receiver, body).await;
            }
        }

        receiver.close().await.unwrap();
        seed_sender.close().await.unwrap();
    }

    async fn qpid_retirement(url: &str) {
        let mut connection = Connection::open("test-connection", url).await.unwrap();
        let mut session = Session::begin(&mut connection).await.unwrap();
        let controller = Controller::attach(&mut session, "test-controller")
            .await
            .unwrap();

        for &body in RETIREMENT_BODIES.iter() {
            let txn = Transaction::declare(&controller, None).await.unwrap();
            qpid_retire_case(body, txn, &mut session).await;
        }

        controller.close().await.unwrap();
        session.close().await.unwrap();
        connection.close().await.unwrap();
    }

    async fn qpid_owned_retirement(url: &str) {
        let mut connection = Connection::open("test-connection", url).await.unwrap();
        let mut session = Session::begin(&mut connection).await.unwrap();

        for (index, &body) in RETIREMENT_BODIES.iter().enumerate() {
            let txn = OwnedTransaction::declare(
                &mut session,
                format!("test-owned-controller-{}", index),
                None,
            )
            .await
            .unwrap();
            qpid_retire_case(body, txn, &mut session).await;
        }

        session.close().await.unwrap();
        connection.close().await.unwrap();
    }

    /// Run one Artemis retirement sub-case: attach the seed sender and receiver for
    /// the case's queue, seed the message, and exercise the retirement method
    /// selected by `body`.
    ///
    /// Artemis only enlists transactional `Accepted` retirements; `Rejected`,
    /// `Released`, and `Modified` transactional outcomes are silently ignored by the
    /// broker, so those cases only verify that the operations complete without an
    /// error and the message's fate is not asserted. An `Accepted` retirement is
    /// reversed on rollback, and consumes the message on commit.
    ///
    /// Each sub-case uses its own queue and receiver so that a late redelivery of an
    /// earlier case cannot interfere with the next one. The receiver is attached
    /// before seeding: on Artemis a message that is enqueued before the consuming
    /// link attaches is not delivered.
    async fn artemis_retire_case<T, R>(
        body: &str,
        txn: T,
        session: &mut SessionHandle<R>,
    ) where
        T: TransactionRetirement + TransactionDischarge + Send + Sync,
        T::RetireError: Debug,
        <T as TransactionDischarge>::Error: Debug,
    {
        let queue = format!("test-queue-ret-{}", body);

        let mut seed_sender = Sender::attach(
            session,
            format!("test-seed-sender-{}", body),
            &queue,
        )
        .await
        .unwrap();
        let mut receiver = Receiver::attach(
            session,
            format!("test-receiver-{}", body),
            &queue,
        )
        .await
        .unwrap();
        seed_sender.send(Message::from(body)).await.unwrap();

        match body {
            "rollback-accept" => {
                retire_accept_rollback(txn, &mut receiver, body).await;
                expect_redelivered(&mut receiver, body).await;
            }
            "accept" => retire_accept_commit(txn, &mut receiver, body).await,
            "reject" => retire_reject_commit(txn, &mut receiver, body).await,
            "release" => retire_release_commit(txn, &mut receiver, body).await,
            "modify" => retire_modify_commit(txn, &mut receiver, body).await,
            _ => retire_outcome_commit(txn, &mut receiver, body).await,
        }

        receiver.close().await.unwrap();
        seed_sender.close().await.unwrap();
    }

    async fn artemis_retirement(url: &str) {
        let mut connection = Connection::open("test-connection", url).await.unwrap();
        let mut session = Session::begin(&mut connection).await.unwrap();
        let controller = Controller::attach(&mut session, "test-controller")
            .await
            .unwrap();

        for &body in RETIREMENT_BODIES.iter() {
            let txn = Transaction::declare(&controller, None).await.unwrap();
            artemis_retire_case(body, txn, &mut session).await;
        }

        controller.close().await.unwrap();
        session.close().await.unwrap();
        connection.close().await.unwrap();
    }

    async fn artemis_owned_retirement(url: &str) {
        let mut connection = Connection::open("test-connection", url).await.unwrap();
        let mut session = Session::begin(&mut connection).await.unwrap();

        for (index, &body) in RETIREMENT_BODIES.iter().enumerate() {
            let txn = OwnedTransaction::declare(
                &mut session,
                format!("test-owned-controller-{}", index),
                None,
            )
            .await
            .unwrap();
            artemis_retire_case(body, txn, &mut session).await;
        }

        session.close().await.unwrap();
        connection.close().await.unwrap();
    }

    // ----- acquisition -----
    //
    // Transactional acquisition is only tested against Qpid Broker-J: Artemis does
    // not implement it (its coordinator only handles declare/discharge and there is
    // no `txn-id` flow handling on the receiving side), so the acquisition scenarios
    // are not run against Artemis.

    async fn acquire_commit_and_verify<T>(txn: T, receiver: &mut Receiver, expected: &str)
    where
        T: TransactionAcquisition,
        <T as TransactionDischarge>::Error: From<FlowError> + Debug,
        <T as TransactionRetirement>::RetireError: Debug,
    {
        let mut txn_acq = txn.acquire(receiver, 1).await.unwrap();
        assert!(!txn_acq.txn_id().as_ref().is_empty());
        let delivery = timeout(Duration::from_secs(5), txn_acq.recv::<String>())
            .await
            .expect("timed out waiting for an acquired delivery")
            .unwrap();
        assert_eq!(delivery.body(), expected);
        txn_acq.accept(&delivery).await.unwrap();
        txn_acq.commit().await.unwrap();
    }

    async fn acquire_rollback_and_verify<T>(txn: T, receiver: &mut Receiver, expected: &str)
    where
        T: TransactionAcquisition,
        <T as TransactionDischarge>::Error: From<FlowError> + Debug,
        <T as TransactionRetirement>::RetireError: Debug,
    {
        let mut txn_acq = txn.acquire(receiver, 1).await.unwrap();
        let delivery = timeout(Duration::from_secs(5), txn_acq.recv::<String>())
            .await
            .expect("timed out waiting for an acquired delivery")
            .unwrap();
        assert_eq!(delivery.body(), expected);
        txn_acq.accept(&delivery).await.unwrap();
        txn_acq.rollback().await.unwrap();
    }

    /// Seed a message, acquire and commit it, and verify that it is consumed.
    async fn acquire_commit_case<T, R>(
        txn: T,
        session: &mut SessionHandle<R>,
        queue: &str,
        body: &str,
    ) where
        T: TransactionAcquisition,
        <T as TransactionDischarge>::Error: From<FlowError> + Debug,
        <T as TransactionRetirement>::RetireError: Debug,
    {
        let mut seed_sender = Sender::attach(session, "test-seed-sender", queue)
            .await
            .unwrap();
        seed_sender.send(Message::from(body)).await.unwrap();
        let mut receiver = Receiver::attach(session, "test-receiver", queue)
            .await
            .unwrap();

        acquire_commit_and_verify(txn, &mut receiver, body).await;

        let mut verify_receiver = Receiver::attach(session, "test-verify-receiver", queue)
            .await
            .unwrap();
        let result = timeout(
            Duration::from_secs(3),
            verify_receiver.recv::<String>(),
        )
        .await;
        assert!(result.is_err(), "acquired message was not consumed by commit");
        verify_receiver.close().await.unwrap();
        receiver.close().await.unwrap();
        seed_sender.close().await.unwrap();
    }

    /// Seed a message, acquire and roll it back, and verify that it is returned
    /// to the queue.
    async fn acquire_rollback_case<T, R>(
        txn: T,
        session: &mut SessionHandle<R>,
        queue: &str,
        body: &str,
    ) where
        T: TransactionAcquisition,
        <T as TransactionDischarge>::Error: From<FlowError> + Debug,
        <T as TransactionRetirement>::RetireError: Debug,
    {
        let mut seed_sender = Sender::attach(session, "test-seed-sender", queue)
            .await
            .unwrap();
        seed_sender.send(Message::from(body)).await.unwrap();
        let mut receiver = Receiver::attach(session, "test-receiver", queue)
            .await
            .unwrap();

        acquire_rollback_and_verify(txn, &mut receiver, body).await;

        let mut verify_receiver = Receiver::attach(session, "test-verify-receiver", queue)
            .await
            .unwrap();
        let delivery = recv_string(&mut verify_receiver).await;
        assert_eq!(delivery.body(), body);
        verify_receiver.accept(&delivery).await.unwrap();
        verify_receiver.close().await.unwrap();
        receiver.close().await.unwrap();
        seed_sender.close().await.unwrap();
    }

    async fn acquisition(url: &str) {
        let mut connection = Connection::open("test-connection", url).await.unwrap();
        let mut session = Session::begin(&mut connection).await.unwrap();
        let controller = Controller::attach(&mut session, "test-controller")
            .await
            .unwrap();

        let txn = Transaction::declare(&controller, None).await.unwrap();
        acquire_commit_case(txn, &mut session, "test-queue-acquire", "acquire-commit").await;

        let txn = Transaction::declare(&controller, None).await.unwrap();
        acquire_rollback_case(
            txn,
            &mut session,
            "test-queue-acquire-rollback",
            "acquire-rollback",
        )
        .await;

        controller.close().await.unwrap();
        session.close().await.unwrap();
        connection.close().await.unwrap();
    }

    async fn owned_acquisition(url: &str) {
        let mut connection = Connection::open("test-connection", url).await.unwrap();
        let mut session = Session::begin(&mut connection).await.unwrap();

        let txn = OwnedTransaction::declare(&mut session, "test-owned-controller-0", None)
            .await
            .unwrap();
        acquire_commit_case(txn, &mut session, "test-queue-acquire", "acquire-commit").await;

        let txn = OwnedTransaction::declare(&mut session, "test-owned-controller-1", None)
            .await
            .unwrap();
        acquire_rollback_case(
            txn,
            &mut session,
            "test-queue-acquire-rollback",
            "acquire-rollback",
        )
        .await;

        session.close().await.unwrap();
        connection.close().await.unwrap();
    }
}
