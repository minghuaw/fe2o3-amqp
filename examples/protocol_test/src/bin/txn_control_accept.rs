use fe2o3_amqp::acceptor::{ConnectionAcceptor, SaslPlainMechanism};
use tokio::net::TcpListener;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;


const BASE_ADDR: &str = "localhost:5672";

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let tcp_listener = TcpListener::bind(BASE_ADDR).await.unwrap();

    // let connection_acceptor = ConnectionAcceptor::new("test_conn_listener");
    let connection_acceptor = ConnectionAcceptor::builder()
        .container_id("example_connection_acceptor")
        .sasl_acceptor(SaslPlainMechanism::new("guest", "guest"))
        .build();

    while let Ok((stream, addr)) = tcp_listener.accept().await {
        
    }

    todo!()
}