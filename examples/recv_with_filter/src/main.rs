use fe2o3_amqp::{
    types::{messaging::Source, primitives::Value},
    Connection, Receiver, Session,
};
use fe2o3_amqp_ext::filters::SelectorFilter;

#[tokio::main]
async fn main() {
    let mut connection = Connection::open("connection-1", "amqp://localhost:5672")
        .await
        .unwrap();
    let mut session = Session::begin(&mut connection).await.unwrap();

    let source = Source::builder()
        .address("q1")
        .add_to_filter("sn", SelectorFilter(String::from("sn = 100")))
        .build();
    let mut receiver = Receiver::builder()
        .name("rust-receiver-link-1")
        .source(source)
        .attach(&mut session)
        .await
        .unwrap();

    let delivery = receiver.recv::<Value>().await.unwrap();
    receiver.accept(&delivery).await.unwrap();
    println!("{:?}", delivery);

    receiver.close().await.unwrap();
    session.end().await.unwrap();
    connection.close().await.unwrap();
}
