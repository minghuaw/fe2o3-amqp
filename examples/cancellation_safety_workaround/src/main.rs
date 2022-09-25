use std::time::Duration;

use fe2o3_amqp::{Connection, Delivery, Receiver, Sender, Session};
use tokio::sync::mpsc::{self, error::TryRecvError};

const NUMBER_OF_MESSAGES: usize = 100_000;
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
        {
            let mut fut = receiver.recv::<String>();
            tokio::pin!(fut);
    
            // This timeout could cause cancellation safety issue as well
            match tokio::time::timeout(Duration::from_millis(1), &mut fut).await {
                Ok(result) => {
                    let delivery = result.unwrap();
                    tokio::spawn(process_delivery(delivery, tx.clone()));
                    // processed_count += try_dispose_processed(&mut rx, &mut receiver).await;
                }
                Err(_) => {
                    todo!()
                }
            };
        };
        
        processed_count += try_dispose_processed(&mut rx, &mut receiver).await;
        // This is for debugging purpose only
        // println!("processed_count: {}", processed_count);

        if processed_count >= NUMBER_OF_MESSAGES {
            println!("processed_count: {}", processed_count);
            break;
        }
    }

    receiver.close().await.unwrap()
}

async fn try_dispose_processed(
    rx: &mut mpsc::Receiver<Delivery<String>>,
    receiver: &mut Receiver,
) -> usize {
    let mut processed_count = 0;
    // Try to see whether there are processed deliveries
    loop {
        match rx.try_recv() {
            Ok(processed_delivery) => {
                receiver.accept(&processed_delivery).await.unwrap();
                processed_count += 1;
            }
            Err(err) => match err {
                TryRecvError::Empty => {
                    // The channel is empty, break out of the loop
                    return processed_count;
                }
                TryRecvError::Disconnected => {
                    // Something went wrong, there should always be at least one copy of `tx` in this task
                    panic!("TryRecvError::Disconnected")
                }
            },
        }
    }
}

async fn process_delivery(delivery: Delivery<String>, tx: mpsc::Sender<Delivery<String>>) {
    use rand::{Rng, SeedableRng};

    // Randomly wait for some time (between 0 to 255 milliseconds) to simulate processing
    let mut rng = rand::rngs::StdRng::from_entropy();
    let millis = rng.gen_range(0..MAX_PROCESSING_TIME_MILLIS);
    tokio::time::sleep(Duration::from_millis(millis)).await;

    // Send back
    // TODO: only `DeliveryInfo` is required for disposition. This is subject to change
    tx.send(delivery).await.unwrap()
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
    tokio::time::timeout(duration, receiver_task_handle)
        .await
        .unwrap()
        .unwrap();

    session.end().await.unwrap();
    connection.close().await.unwrap();
}
