# fe2o3-amqp

A rust implementation of AMQP 1.0 protocol based on serde and tokio.

[![crate_version](https://img.shields.io/crates/v/fe2o3-amqp.svg?style=flat)](https://crates.io/crates/fe2o3-amqp) [![docs_version](https://img.shields.io/badge/docs-latest-blue.svg?style=flat)](https://docs.rs/fe2o3-amqp/latest/fe2o3_amqp/)

## Feature flags

```toml
default = []
```

| Feature | Description |
|---------|-------------|
|`"rustls"`| enables TLS integration with `tokio-rustls` and `rustls` |
|`"native-tls"`|enables TLS integration with `tokio-native-tls` and `native-tls`|
|`"acceptor"`|enables `ConnectionAcceptor`, `SessionAcceptor`, and `LinkAcceptor`|
|`"transaction"`| enables `Controller`, `Transaction`, `TxnAcquisition` (only the client side is implemented so far) |

## Quick start

1. [Client](#client)
2. [Listener](#listener)

### Client

Below is an example with a local broker (
[`TestAmqpBroker`](https://github.com/Azure/amqpnetlite/releases/download/test_broker.1609/TestAmqpBroker.zip))
listening on the localhost. The broker is executed with the following command

```powershell
./TestAmqpBroker.exe amqp://localhost:5672 /creds:guest:guest /queues:q1
```

The following code requires the [`tokio`] async runtime added to the dependencies.

```rust
use fe2o3_amqp::{Connection, Session, Sender, Receiver};

#[tokio::main]
async fn main() {
    let mut connection = Connection::open(
        "connection-1",                     // container id
        "amqp://guest:guest@localhost:5672" // url
    ).await.unwrap();

    let mut session = Session::begin(&mut connection).await.unwrap();

    // Create a sender
    let mut sender = Sender::attach(
        &mut session,           // Session
        "rust-sender-link-1",   // link name
        "q1"                    // target address
    ).await.unwrap();

    // Create a receiver
    let mut receiver = Receiver::attach(
        &mut session,
        "rust-receiver-link-1", // link name
        "q1"                    // source address
    ).await.unwrap();

    // Send a message to the broker and wait for outcome (Disposition)
    let outcome: Outcome = sender.send("hello AMQP").await.unwrap();
    outcome.accepted_or_else(|state| state).unwrap(); // Handle delivery outcome

    // Send a message
    let fut = sender.send_batchable("hello batchable AMQP").await.unwrap();
    // Wait for outcome (Disposition)
    let outcome: Outcome = fut.await.unwrap();
    outcome.accepted_or_else(|state| state).unwrap(); // Handle delivery outcome

    // Receive the message from the broker
    let delivery = receiver.recv::<String>().await.unwrap();
    receiver.accept(&delivery).await.unwrap();

    // Detach links with closing Detach performatives
    sender.close().await.unwrap();
    receiver.close().await.unwrap();

    // End the session
    session.end().await.unwrap();

    // Close the connection
    connection.close().await.unwrap();
}
```

### Listener

```rust
use tokio::net::TcpListener;
use fe2o3_amqp::acceptor::{ConnectionAcceptor, SessionAcceptor, LinkAcceptor, LinkEndpoint};

#[tokio::main]
async fn main() {
    let tcp_listener = TcpListener::bind("localhost:5672").await.unwrap();
    let connection_acceptor = ConnectionAcceptor::new("example-listener");

    while let Ok((stream, addr)) = tcp_listener.accept().await {
        let mut connection = connection_acceptor.accept(stream).await.unwrap();
        let handle = tokio::spawn(async move {
            let session_acceptor = SessionAcceptor::new();
            while let Ok(mut session) = session_acceptor.accept(&mut connection).await{
                let handle = tokio::spawn(async move {
                    let link_acceptor = LinkAcceptor::new();
                    match link_acceptor.accept(&mut session).await.unwrap() {
                        LinkEndpoint::Sender(sender) => { },
                        LinkEndpoint::Receiver(recver) => { },
                    }
                });
            }
        });
    }
}
```

## More examples

More examples of sending and receiving can be found on the [GitHub repo](https://github.com/minghuaw/fe2o3-amqp/tree/main/examples/).
Please note that most examples requires a local broker running. One broker that can be used on Windows is [TestAmqpBroker](https://azure.github.io/amqpnetlite/articles/hello_amqp.html).

## Components

| Name | Description |
|------|-------------|
|`serde_amqp_derive`| Custom derive macro for described types as defined in AMQP1.0 protocol |
|`serde_amqp`| AMQP1.0 serializer and deserializer as well as primitive types |
|`fe2o3-amqp-types`| AMQP1.0 data types |
|`fe2o3-amqp`| Implementation of AMQP1.0 `Connection`, `Session`, and `Link` |
|`fe2o3-amqp-ext`| Extension types and implementations |

### Road map

The items below are listed in the order of priority.

- [x] Proper error handling (more or less)
- [x] Listeners
  - [x] Acceptor that provide fine control over each incoming endpoint
  - [x] TLS acceptor integration with `tokio-rustls`
  - [x] TLS acceptor integration with `tokio-native-tls`
  - [x] Naive PLAIN SASL acceptor
  - [ ] ~~Listener that provide coarse control~~
- [x] Transaction
  - [x] controller side
  - [x] controller side testing
    - [x] posting
    - [x] retirement
    - [ ] ~~acquisition~~ #43
  - [x] resource side and testing
    - [x] posting
    - [x] retirement
    - [ ] ~~acquisition~~ #43
- [x] [qpid interoperability test](https://github.com/minghuaw/qpid-interop-test)
- [x] Link resumption
- [ ] Dynamic link creation
- [ ] Message batch disposition
- [ ] Pipelined open
- [ ] SASL SCRAM-SHA1
  - [ ] acceptor

License: MIT/Apache-2.0
