use fe2o3_amqp::{
    acceptor::{
        ConnectionAcceptor, LinkAcceptor, LinkEndpoint, ListenerConnectionHandle,
        ListenerSessionHandle, SessionAcceptor,
    },
    transaction::coordinator::ControlLinkAcceptor,
    types::primitives::Value,
    Receiver, Sendable, Sender,
};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let tcp_listener = TcpListener::bind("localhost:5672").await.unwrap();
    let connection_acceptor = ConnectionAcceptor::new("example-connection-acceptor");

    while let Ok((stream, addr)) = tcp_listener.accept().await {
        println!("Incoming connection from {}", addr);
        let connection = connection_acceptor.accept(stream).await.unwrap();

        let _ = tokio::spawn(connection_main(connection));
    }
}

async fn connection_main(mut connection: ListenerConnectionHandle) {
    let session_acceptor = SessionAcceptor::builder()
        .control_link_acceptor(ControlLinkAcceptor::default()) // This enables the session acceptor to accept control link
        .build();

    while let Ok(session) = session_acceptor.accept(&mut connection).await {
        let _ = tokio::spawn(session_main(session));
    }
}

async fn session_main(mut session: ListenerSessionHandle) {
    let _ = tokio::spawn(async move {
        let link_acceptor = LinkAcceptor::new();

        while let Ok(link) = link_acceptor.accept(&mut session).await {
            match link {
                LinkEndpoint::Sender(sender) => {
                    let _ = tokio::spawn(sender_main(sender));
                }
                LinkEndpoint::Receiver(receiver) => {
                    let _ = tokio::spawn(receiver_main(receiver));
                }
            }
        }
    });
}

async fn sender_main(mut sender: Sender) {
    let sendable = Sendable::builder()
        .message("hello world")
        .settled(false)
        .build();
    let outcome = sender.send(sendable).await.unwrap();
    outcome.accepted_or_else(|outcome| outcome).unwrap();

    sender.on_detach().await;
    sender.close().await.unwrap();
}

async fn receiver_main(mut receiver: Receiver) {
    loop {
        let delivery = receiver.recv::<Value>().await.unwrap();
        receiver.accept(&delivery).await.unwrap();
        println!("{:?}", delivery.body());
    }
}
