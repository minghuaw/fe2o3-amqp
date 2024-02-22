use fe2o3_amqp::{
    transaction::{Controller, Transaction, TransactionDischarge, TransactionalRetirement},
    types::primitives::Value,
    Connection, Delivery, Receiver, Session,
};

#[tokio::main]
async fn main() {
    let mut connection = Connection::open("connection-1", "amqp://localhost:5672")
        .await
        .unwrap();
    let mut session = Session::begin(&mut connection).await.unwrap();
    let mut receiver = Receiver::attach(&mut session, "rust-recver-1", "q1")
        .await
        .unwrap();

    let mut controller = Controller::attach(&mut session, "controller-1")
        .await
        .unwrap();

    let delivery: Delivery<Value> = receiver.recv().await.unwrap();

    // Transactionally retiring
    let txn = Transaction::declare(&mut controller, None).await.unwrap();
    txn.accept(&mut receiver, &delivery).await.unwrap();
    txn.commit().await.unwrap();

    controller.close().await.unwrap();
    receiver.close().await.unwrap();
    session.close().await.unwrap();
    connection.close().await.unwrap();
}
