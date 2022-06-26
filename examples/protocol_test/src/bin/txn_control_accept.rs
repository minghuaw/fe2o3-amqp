use fe2o3_amqp::{
    acceptor::{
        ConnectionAcceptor, LinkAcceptor, LinkEndpoint, SaslPlainMechanism, SessionAcceptor,
    },
    types::primitives::Value, transaction::coordinator::ControlLinkAcceptor,
};
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

    let connection_acceptor = ConnectionAcceptor::new("test_conn_listener");
    // let connection_acceptor = ConnectionAcceptor::builder()
    //     .container_id("example_connection_acceptor")
    //     .sasl_acceptor(SaslPlainMechanism::new("guest", "guest"))
    //     .build();

    while let Ok((stream, addr)) = tcp_listener.accept().await {
        tracing::info!("Incoming connection from {:?}", addr);
        let mut connection = connection_acceptor.accept(stream).await.unwrap();

        let _ = tokio::spawn(async move {
            let session_acceptor = SessionAcceptor::builder()
                .control_link_acceptor(ControlLinkAcceptor::default())
                .build();

            while let Ok(mut session) = session_acceptor.accept(&mut connection).await {
                let _ = tokio::spawn(async move {
                    tracing::info!("Incoming session created");

                    let link_acceptor = LinkAcceptor::new();

                    while let Ok(link) = link_acceptor.accept(&mut session).await {
                        match link {
                            LinkEndpoint::Sender(mut sender) => {
                                let _ = tokio::spawn(async move {
                                    tracing::info!("Incoming link is connected (remote: receiver, local: sender)");
                                    sender.send("world").await.unwrap();
                                    if let Err(e) = sender.close().await {
                                        // The remote may close the session
                                        tracing::error!(link="sender", error=?e);
                                    }
                                });
                            }
                            LinkEndpoint::Receiver(mut recver) => {
                                let _ = tokio::spawn(async move {
                                    tracing::info!("Incoming link is connected (remote: sender, local: receiver");
                                    let delivery = recver.recv::<Value>().await.unwrap();
                                    tracing::info!(message = ?delivery.message());
                                    recver.accept(&delivery).await.unwrap();
                                    if let Err(e) = recver.close().await {
                                        // The remote may close the session
                                        tracing::error!(link="receiver", error=?e);
                                    }
                                });
                            }
                        }
                    }
                });
            }
        });
    }
}
