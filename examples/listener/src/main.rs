use fe2o3_amqp::{
    acceptor::{
        link::{LinkAcceptor, LinkEndpoint},
        session::{ListenerSessionHandle, SessionAcceptor},
        ConnectionAcceptor, ListenerConnectionHandle,
    },
    types::primitives::Value,
    Receiver, Sender,
};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let tcp_listener = TcpListener::bind("localhost:5672").await.unwrap();
    let connection_acceptor = ConnectionAcceptor::new("example_connection_acceptor");

    while let Ok((stream, addr)) = tcp_listener.accept().await {
        println!("Incoming connection from {:?}", addr);
        let connection = connection_acceptor.accept(stream).await.unwrap();

        let _ = tokio::spawn(connection_main(connection));
    }
}

async fn connection_main(mut connection: ListenerConnectionHandle) {
    let session_acceptor = SessionAcceptor::default();

    while let Ok(session) = session_acceptor.accept(&mut connection).await {
        let _ = tokio::spawn(session_main(session));
    }
    connection.on_close().await.unwrap();
}

async fn session_main(mut session: ListenerSessionHandle) {
    let link_acceptor = LinkAcceptor::new();

    while let Ok(link) = link_acceptor.accept(&mut session).await {
        match link {
            LinkEndpoint::Sender(sender) => tokio::spawn(sender_main(sender)),
            LinkEndpoint::Receiver(receiver) => tokio::spawn(receiver_main(receiver)),
        };
    }

    session.on_end().await.unwrap();
}

async fn sender_main(mut sender: Sender) {
    sender.send("hello world").await.unwrap();
    sender.close().await.unwrap();
}

async fn receiver_main(mut receiver: Receiver) {
    while let Ok(delivery) = receiver.recv::<Value>().await {
        receiver.accept(&delivery).await.unwrap();
        println!("{:?}", delivery.body())
    }
    receiver.close().await.unwrap();
}
