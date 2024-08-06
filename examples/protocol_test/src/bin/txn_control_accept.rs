use fe2o3_amqp::{
    acceptor::{
        ConnectionAcceptor, LinkAcceptor, LinkEndpoint, ListenerConnectionHandle,
        ListenerSessionHandle, SessionAcceptor, SupportedReceiverSettleModes,
    },
    transaction::coordinator::ControlLinkAcceptor,
    types::primitives::Value,
    Receiver, Sendable, Sender,
};
use tokio::net::TcpListener;
use tracing::{instrument, Level};
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
        let connection = connection_acceptor.accept(stream).await.unwrap();

        let _ = tokio::spawn(connection_main(connection));
    }
}

async fn connection_main(mut connection: ListenerConnectionHandle) {
    let session_acceptor = SessionAcceptor::builder()
        .control_link_acceptor(
            ControlLinkAcceptor::builder()
                .supported_receiver_settle_modes(SupportedReceiverSettleModes::First)
                .build(),
        )
        .build();

    while let Ok(session) = session_acceptor.accept(&mut connection).await {
        let _ = tokio::spawn(session_main(session));
    }
}

async fn session_main(mut session: ListenerSessionHandle) {
    let _ = tokio::spawn(async move {
        tracing::info!("Incoming session created");

        let link_acceptor = LinkAcceptor::new();

        while let Ok(link) = link_acceptor.accept(&mut session).await {
            tracing::info!("New link endpoint");
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
    // let mut interval = tokio::time::interval(Duration::from_millis(500));
    // loop {
    //     interval.tick().await;
    //     match sender.send("hello AMQP").await {
    //         Ok(_) => {},
    //         Err(error) => {
    //             tracing::error!(?error);
    //             sender.close().await.unwrap();
    //             return
    //         },
    //     }
    // }

    let sendable = Sendable::builder()
        .message("hello world")
        .settled(false)
        .build();
    let fut1 = sender.send_batchable(sendable).await.unwrap();
    // tokio::spawn(async move {
    //     let first = fut1.await;
    //     // let second = fut2.await;
    //     tracing::info!(?first);
    //     // tracing::info!(?second);
    // });

    let sendable = Sendable::builder()
        .message("foo bar")
        .settled(false)
        .build();
    let fut2 = sender.send_batchable(sendable).await.unwrap();
    // tokio::spawn(async move {
    //     let second = fut2.await;
    //     tracing::info!(?second);
    // });

    let (first, second) = tokio::join!(fut1, fut2);
    tracing::info!(?first);
    tracing::info!(?second);

    let detached = sender.on_detach().await;
    tracing::info!(?detached);
    // let result = tokio::time::timeout(Duration::from_millis(500), handle).await;
    // tracing::info!(?result);
    sender.close().await.unwrap();
}

#[instrument(skip_all)]
async fn receiver_main(mut receiver: Receiver) {
    loop {
        match receiver.recv::<Value>().await {
            Ok(delivery) => {
                // tracing::info!(body = ?delivery.body());
                match receiver.accept(&delivery).await {
                    Ok(outcome) => {
                        tracing::info!(?outcome)
                    }
                    Err(error) => {
                        tracing::error!(?error);
                        receiver.close().await.unwrap();
                        return;
                    }
                }
            }
            Err(error) => {
                tracing::error!(?error);
                receiver.close().await.unwrap();
                return;
            }
        }
    }
}
