use fe2o3_amqp::{listener::ConnectionAcceptor, Connection};
use tokio::net::TcpListener;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

const BASE_ADDR: &str = "localhost:5671";

async fn listener_main() {
    let tcp_listener = TcpListener::bind(BASE_ADDR).await.unwrap();
    let connection_acceptor = ConnectionAcceptor::builder()
        .container_id("test_listener")
        .build();

    loop {
        let (stream, addr) = tcp_listener.accept().await.unwrap();
        println!("Incoming connection from {:?}", addr);
        let mut connection = connection_acceptor.accept(stream).await.unwrap();
        let result = connection.close().await;
        println!("{:?}", result);
    }
}

async fn client_main() {
    let url = format!("amqp://{}", BASE_ADDR);
    let mut connection = Connection::open("connection-1", &url[..]).await.unwrap();
    connection.close().await.unwrap();
}

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let handle = tokio::spawn(listener_main());
    client_main().await;

    handle.abort();
    let _ = handle.await;
}
