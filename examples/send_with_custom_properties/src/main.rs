use fe2o3_amqp::{
    types::messaging::{ApplicationProperties, Message, Properties},
    Connection, Sender, Session,
};

#[tokio::main]
async fn main() {
    let mut connection = Connection::open("sender-connection", "amqp://localhost:5672")
        .await
        .unwrap();
    let mut session = Session::begin(&mut connection).await.unwrap();

    let mut sender = Sender::attach(&mut session, "rust-sender-link-1", "q1")
        .await
        .unwrap();

    let message = Message::builder()
        .properties(Properties::builder().message_id(1).build())
        .application_properties(ApplicationProperties::builder().insert("sn", 100).build())
        .value("hellow AMQP")
        .build();

    let outcome = sender.send(message).await.unwrap();
    outcome.accepted_or_else(|outcome| outcome).unwrap();

    sender.close().await.unwrap();
    session.end().await.unwrap();
    connection.close().await.unwrap();
}
