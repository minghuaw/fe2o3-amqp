use fe2o3_amqp::{
    transaction::{OwnedTransaction, TransactionDischarge},
    Connection, Sender, Session,
};

#[tokio::main]
async fn main() {
    let mut connection = Connection::open("connection-1", "amqp://localhost:5672")
        .await
        .unwrap();
    let mut session = Session::begin(&mut connection).await.unwrap();
    let mut sender = Sender::attach(&mut session, "rust-sender-link-1", "q1")
        .await
        .unwrap();

    // Commit
    let mut txn1 = OwnedTransaction::declare(&mut session, "controller-1", None)
        .await
        .unwrap();
    txn1.post(&mut sender, "hello").await.unwrap();
    txn1.post(&mut sender, "world").await.unwrap();
    txn1.commit().await.unwrap();

    // Rollback
    let mut txn2 = OwnedTransaction::declare(&mut session, "controller-2", None)
        .await
        .unwrap();
    txn2.post(&mut sender, "foo").await.unwrap();
    txn2.rollback().await.unwrap();

    sender.close().await.unwrap();
    session.close().await.unwrap();
    connection.close().await.unwrap();
}
