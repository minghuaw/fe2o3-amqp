use std::vec;

use fe2o3_amqp::{listener::{ConnectionAcceptor, session::SessionAcceptor}};
use tokio::net::TcpListener;
use tracing::{Level, instrument};
use tracing_subscriber::FmtSubscriber;

const BASE_ADDR: &str = "localhost:5671";

#[instrument]
async fn connection_main() {
    let tcp_listener = TcpListener::bind(BASE_ADDR).await.unwrap();
    let connection_acceptor = ConnectionAcceptor::new("test_conn_listener");
    let session_acceptor = SessionAcceptor::default();
    let mut sessions = Vec::new();

    loop {
        let (stream, addr) = tcp_listener.accept().await.unwrap();
        println!("Incoming connection from {:?}", addr);
        let mut connection = connection_acceptor.accept(stream).await.unwrap();

        for _ in 0..2 {
            if let Ok(session) = session_acceptor.accept(&mut connection).await {
                sessions.push(session);
            }
        }
        
        for mut session in sessions.drain(..) {
            let result = session.close().await;
            println!("Session close result: {:?}", result);
        }

        let result = connection.close().await;
        println!("Connection close result: {:?}", result);
    }
}

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let handle = tokio::spawn(connection_main());

    handle.await.unwrap();
}
