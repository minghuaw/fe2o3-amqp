use fe2o3_amqp::{
    connection::Connection,
    link::Receiver,
    sasl_profile::SaslProfile,
    session::Session,
    types::{primitives::Value},
    Delivery,
};
use tokio::net::TcpStream;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    tracing::info!("Starting connection");

    // let addr = "localhost:5671";
    // let domain = "localhost";
    // let stream = TcpStream::connect(addr).await.unwrap();
    // let connector = native_tls::TlsConnector::builder()
    //     .danger_accept_invalid_certs(true)
    //     .build()
    //     .unwrap();
    // let connector = tokio_native_tls::TlsConnector::from(connector);
    // let tls_stream = connector.connect(domain, stream).await.unwrap();

    let mut connection = Connection::builder()
        .container_id("connection-1")
        .scheme("amqp")
        // .max_frame_size(1000)
        .channel_max(9)
        .idle_time_out(50_000 as u32)
        // .sasl_profile(SaslProfile::Plain {
        //     username: "guest".into(),
        //     password: "guest".into(),
        // })
        // .open_with_stream(tls_stream)
        .open("amqp://localhost:5672")
        // .open("amqp://guest:guest@localhost:5672")
        .await
        .unwrap();

    let mut session = Session::begin(&mut connection).await.unwrap();

    let mut receiver = Receiver::attach(&mut session, "rust-recver-1", "q1")
        .await
        .unwrap();
    // let mut receiver = Receiver::builder()
    //     .name("rust-receiver-link-1")
    //     .source("q1")
    //     .attach(&mut session)
    //     .await
    //     .unwrap();

    println!("Receiver attached");
    // tokio::time::sleep(Duration::from_millis(500)).await;

    let delivery: Delivery<Value> = receiver.recv().await.unwrap();
    receiver.accept(&delivery).await.unwrap();
    println!("{:?}", delivery.delivery_id());

    let delivery = receiver.recv::<Value>().await.unwrap();
    receiver.accept(&delivery).await.unwrap();
    // let body = delivery.into_body();
    println!("{:?}", delivery.delivery_id());

    receiver.close().await.unwrap();
    session.end().await.unwrap();
    connection.close().await.unwrap();
}
