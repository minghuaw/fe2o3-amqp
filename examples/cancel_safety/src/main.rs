use std::time::Duration;

use fe2o3_amqp::{Connection, Delivery, Receiver, Sender, Session, link::delivery::DeliveryInfo};
use tokio::sync::mpsc;

const NUMBER_OF_MESSAGES: usize = 1_000_000;
const MAX_PROCESSING_TIME_MILLIS: u64 = 10; // in milliseconds

async fn sender_task(mut sender: Sender) {
    let mut outcome_futs = Vec::new();

    // Send with batchable set to true
    for _ in 0..NUMBER_OF_MESSAGES {
        let fut = sender.send_batchable("hello AMQP").await.unwrap();
        outcome_futs.push(fut);
    }

    for fut in outcome_futs {
        let outcome = fut.await.unwrap();
        outcome.accepted_or_else(|outcome| outcome).unwrap();
    }

    sender.close().await.unwrap()
}

async fn receiver_task(mut receiver: Receiver) {
    let (tx, mut rx) = mpsc::channel(NUMBER_OF_MESSAGES);
    let mut processed_count = 0;

    loop {
        tokio::select! {
            result = receiver.recv::<String>() => {
                let delivery = result.unwrap();
                tokio::spawn(process_delivery(delivery, tx.clone()));
            },
            processed = rx.recv() => {
                let delivery_info = processed.unwrap();
                receiver.accept(delivery_info).await.unwrap();
                processed_count += 1;
            }
        }

        if processed_count >= NUMBER_OF_MESSAGES {
            println!("processed_count: {}", processed_count);
            break;
        }
    }

    receiver.close().await.unwrap()
}

async fn process_delivery(delivery: Delivery<String>, tx: mpsc::Sender<DeliveryInfo>) {
    use rand::{Rng, SeedableRng};

    // Randomly wait for some time (between 0 to MAX_PROCESSING_TIME_MILLIS) to simulate processing
    let mut rng = rand::rngs::StdRng::from_entropy();
    let millis = rng.gen_range(0..MAX_PROCESSING_TIME_MILLIS);
    tokio::time::sleep(Duration::from_millis(millis)).await;

    // Send back
    // TODO: only `DeliveryInfo` is required for disposition. This is subject to change
    tx.send(DeliveryInfo::from(delivery)).await.unwrap()
}

#[tokio::main]
async fn main() {
    let mut connection = Connection::open("connection-1", "amqp://localhost:5672")
        .await
        .unwrap();
    let mut session = Session::begin(&mut connection).await.unwrap();
    let sender = Sender::attach(&mut session, "rust-sender-link-1", "q1")
        .await
        .unwrap();
    let receiver = Receiver::attach(&mut session, "rust-receiver-link-1", "q1")
        .await
        .unwrap();

    let receiver_task_handle = tokio::spawn(receiver_task(receiver));
    let sender_task_handle = tokio::spawn(sender_task(sender));

    sender_task_handle.await.unwrap();
    // The maximum wait time should be around NUMBER_OF_MESSAGES * maximum_processing_time_per_delivery.
    // Add an additional 10 secs to be safe
    let duration =
        Duration::from_millis(NUMBER_OF_MESSAGES as u64 * u8::MAX as u64) + Duration::from_secs(10);
    // If the receiver failed to receive any message because of cancel safety issue, the receiver_task will
    // never finish within the maximum test duration
    tokio::time::timeout(duration, receiver_task_handle)
        .await
        .unwrap()
        .unwrap();

    session.end().await.unwrap();
    connection.close().await.unwrap();
}
