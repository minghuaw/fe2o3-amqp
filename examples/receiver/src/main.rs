use fe2o3_amqp::{
    connection::Connection, link::Receiver, session::Session, types::primitives::Value, Delivery,
    link::RecvError,
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

    let delivery: Delivery<Value> = match receiver.recv().await {
        Ok(delivery) => delivery,
        Err(e) => match e {
            RecvError::MessageDecode(e) => {
                receiver.reject(e.info, None).await.unwrap();
                panic!("Message decode error: {:?}", e.source)
            },
            _ => panic!("Unexpected error: {:?}", e),
        }
    };
    receiver.accept(&delivery).await.unwrap();

    receiver.close().await.unwrap();
    session.end().await.unwrap();
    connection.close().await.unwrap();
}
