use std::vec;

use fe2o3_amqp::{listener::{ConnectionAcceptor, session::{SessionAcceptor, ListenerSessionHandle}, link::{LinkAcceptor, LinkEndpoint}, ListenerConnectionHandle}};
use tokio::net::TcpListener;
use tracing::{Level, instrument};
use tracing_subscriber::FmtSubscriber;

const BASE_ADDR: &str = "localhost:5671";

#[instrument(skip_all)]
async fn session_main(mut session: ListenerSessionHandle) {
    let link_acceptor = LinkAcceptor::new();

    loop {
        let link = link_acceptor.accept(&mut session).await.unwrap();
        match link {
            LinkEndpoint::Sender(sender) => sender.close().await.unwrap(),
            LinkEndpoint::Receiver(recver) => recver.close().await.unwrap(),
        }
    }
}

#[instrument(skip_all)]
async fn connection_main(mut connection: ListenerConnectionHandle) {
    let session_acceptor = SessionAcceptor::default();

    loop {
        if let Ok(session) = session_acceptor.accept(&mut connection).await {
            let _handle = tokio::spawn(session_main(session));
        }
    }
}

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let tcp_listener = TcpListener::bind(BASE_ADDR).await.unwrap();
    let connection_acceptor = ConnectionAcceptor::new("test_conn_listener");
    
    loop {
        let (stream, addr) = tcp_listener.accept().await.unwrap();
        println!("Incoming connection from {:?}", addr);
        let connection = connection_acceptor.accept(stream).await.unwrap();

        let _handle = tokio::spawn(connection_main(connection));
    }
}
