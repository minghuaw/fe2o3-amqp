use fe2o3_amqp::{Connection, Session, Sender, Receiver};
use fe2o3_amqp::types::messaging::Outcome;

#[tokio::main]
async fn main() {
    let mut connection = Connection::open(
        "connection-1",                     // container id
        "amqp://guest:guest@localhost:5672" // url
    ).await.unwrap();

    let mut session = Session::begin(&mut connection).await.unwrap();

    // Create a sender
    let mut sender = Sender::attach(
        &mut session,           // Session
        "rust-sender-link-1",   // link name
        "q1"                    // target address
    ).await.unwrap();

    // Create a receiver
    let mut receiver = Receiver::attach(
        &mut session,
        "rust-receiver-link-1", // link name
        "q1"                    // source address
    ).await.unwrap();

    // Send a message to the broker and wait for outcome (Disposition)
    let outcome: Outcome = sender.send("hello AMQP").await.unwrap();
    outcome.accepted_or_else(|state| state).unwrap(); // Handle delivery outcome

    // Send a message with batchable field set to true
    let fut = sender.send_batchable("hello batchable AMQP").await.unwrap();
    let outcome: Outcome = fut.await.unwrap(); // Wait for outcome (Disposition)
    outcome.accepted_or_else(|state| state).unwrap(); // Handle delivery outcome

    // Receive the message from the broker
    let delivery = receiver.recv::<String>().await.unwrap();
    receiver.accept(&delivery).await.unwrap();

    let delivery = receiver.recv::<String>().await.unwrap();
    receiver.accept(&delivery).await.unwrap();

    sender.detach().await.unwrap(); // Detach sender with closing Detach performatives
    receiver.detach().await.unwrap(); // Detach receiver with closing Detach performatives
    session.end().await.unwrap(); // End the session
    connection.close().await.unwrap(); // Close the connection
}