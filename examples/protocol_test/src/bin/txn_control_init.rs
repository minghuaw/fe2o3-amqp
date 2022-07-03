use std::time::Duration;

use fe2o3_amqp::{
    transaction::{Controller, Transaction, TransactionDischarge, OwnedTransaction, TransactionalRetirement, coordinator::ControlLinkAcceptor},
    types::{primitives::Value, definitions::ReceiverSettleMode},
    Connection, Delivery, Receiver, Sender, Session, Sendable,
};
use tracing::{instrument, Level};
use tracing_subscriber::FmtSubscriber;

const SASL_PLAIN: &str = "";
// const SASL_PLAIN: &str = "guest:guest";
const BASE_ADDR: &str = "localhost:5672";

// #[instrument]
async fn client_main() {
    let url = format!("amqp://{}@{}", SASL_PLAIN, BASE_ADDR);

    let mut connection = Connection::open("connection-1", &url[..]).await.unwrap();
    // let mut session = Session::begin(&mut connection).await.unwrap();
    let mut session = Session::builder().begin(&mut connection).await.unwrap();

    // let controller = Controller::attach(&mut session, "controller").await.unwrap();
    let mut txn = OwnedTransaction::declare(&mut session, "owned-controller", None).await.unwrap();
    // let mut txn = Transaction::declare(&controller, None).await.unwrap();

    // Test a regular receiver
    // let mut receiver = Receiver::attach(&mut session, "receiver-1", "q1")
    //     .await
    //     .unwrap();
    let mut receiver = Receiver::builder()
        .name("receiver-1")
        .source("q1")
        .receiver_settle_mode(ReceiverSettleMode::Second)
        .attach(&mut session)
        .await
        .unwrap();

    let delivery1: Delivery<Value> = receiver.recv().await.unwrap();
    tracing::info!(message = ?delivery1.message());
    
    txn.accept(&mut receiver, &delivery1).await.unwrap();

    let delivery2: Delivery<Value> = receiver.recv().await.unwrap();
    tracing::info!(message = ?delivery2.message());
    
    txn.accept(&mut receiver, &delivery2).await.unwrap();

    // Test a regular sender
    // let mut sender = Sender::attach(&mut session, "sender-1", "q1")
    //     .await
    //     .unwrap();
    // sender.send("hello AMQP").await.unwrap();

    // // Test creating a control link
    // match Controller::attach(&mut session, "controller").await {
    //     Ok(mut controller) => {
    //         let mut txn = Transaction::declare(&mut controller, None).await.unwrap();

    //         tracing::info!("Transaction declared");

    //         let sendable = Sendable::builder()
    //             .message("Hello World")
    //             .settled(false)
    //             .build();
    //         let fut1 = txn.post_batchable(&mut sender, sendable).await.unwrap();
    //         fut1.await.unwrap();

    //         let sendable = Sendable::builder()
    //             .message("Foo Bar")
    //             .settled(false)
    //             .build();
    //         let fut2 = txn.post_batchable(&mut sender, sendable).await.unwrap();
    //         fut2.await.unwrap();
                
    //         txn.commit().await.unwrap();

    //         controller.close().await.unwrap();
    //     }
    //     Err(attach_error) => {
    //         tracing::error!(?attach_error)
    //     }
    // }

    
    // txn.post(&mut sender, "Hello World").await.unwrap();
    // txn.post(&mut sender, "Foo Bar").await.unwrap();
    txn.rollback().await.unwrap();
    receiver.reject(&delivery1, None).await.unwrap();
    receiver.reject(&delivery2, None).await.unwrap();
    
    // tracing::info!("closing control link");
    // controller.close().await.unwrap();


    tracing::info!("closing receiver");
    // sender.close().await.unwrap();
    receiver.close().await.unwrap();

    tracing::info!("closing session");
    session.end().await.unwrap();
    connection.close().await.unwrap();
}

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    client_main().await;
}
