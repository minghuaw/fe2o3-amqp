#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs, missing_debug_implementations)]
#![warn(clippy::unused_async)]

//! A rust implementation of AMQP 1.0 protocol based on serde and tokio.
//!
//! [![crate_version](https://img.shields.io/crates/v/fe2o3-amqp.svg?style=flat)](https://crates.io/crates/fe2o3-amqp) [![docs_version](https://img.shields.io/badge/docs-latest-blue.svg?style=flat)](https://docs.rs/fe2o3-amqp/latest/fe2o3_amqp/)
//!
//! - [Documentation](https://docs.rs/fe2o3-amqp)
//! - [Changelog](https://github.com/minghuaw/fe2o3-amqp/blob/main/fe2o3-amqp/Changelog.md)
//! - [0.7 migration guide](https://github.com/minghuaw/fe2o3-amqp/issues/120)
//!
//! # Feature flags
//!
//! ```toml
//! default = []
//! ```
//!
//! | Feature | Description |
//! |---------|-------------|
//! |`"rustls"`| enables TLS integration with `tokio-rustls` and `rustls` |
//! |`"native-tls"`| enables TLS integration with `tokio-native-tls` and `native-tls`|
//! |`"acceptor"`| enables `ConnectionAcceptor`, `SessionAcceptor`, and `LinkAcceptor`|
//! |`"transaction"`| enables `Controller`, `Transaction`, `OwnedTransaction` and `control_link_acceptor` |
//! |`"scram"`| enables SCRAM auth |
//! |`"tracing"`| enables logging with `tracing` |
//! |`"log"`| enables logging with `log` |
//!
//! # Quick start
//!
//! 1. [Client](#client)
//! 2. [Listener](#listener)
//! 3. [WebSocket binding](#websocket)
//!
//! More examples including one showing how to use it with Azure Serivce Bus can be found on the [GitHub repo](https://github.com/minghuaw/fe2o3-amqp/tree/main/examples).
//!
//! ## Client
//!
//! Below is an example with a local broker ([`TestAmqpBroker`](https://github.com/Azure/amqpnetlite/releases/download/test_broker.1609/TestAmqpBroker.zip))
//! listening on the localhost. The broker is executed with the following command
//!
//! ```powershell
//! ./TestAmqpBroker.exe amqp://localhost:5672 /creds:guest:guest /queues:q1
//! ```
//!
//! The following code requires the [`tokio`] async runtime added to the dependencies.
//!
//! ```rust
//! use fe2o3_amqp::{Connection, Session, Sender, Receiver};
//! use fe2o3_amqp::types::messaging::Outcome;
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut connection = Connection::open(
//!         "connection-1",                     // container id
//!         "amqp://guest:guest@localhost:5672" // url
//!     ).await.unwrap();
//!     
//!     let mut session = Session::begin(&mut connection).await.unwrap();
//!     
//!     // Create a sender
//!     let mut sender = Sender::attach(
//!         &mut session,           // Session
//!         "rust-sender-link-1",   // link name
//!         "q1"                    // target address
//!     ).await.unwrap();
//!     
//!     // Create a receiver
//!     let mut receiver = Receiver::attach(
//!         &mut session,
//!         "rust-receiver-link-1", // link name
//!         "q1"                    // source address
//!     ).await.unwrap();
//!     
//!     // Send a message to the broker and wait for outcome (Disposition)
//!     let outcome: Outcome = sender.send("hello AMQP").await.unwrap();
//!     outcome.accepted_or_else(|state| state).unwrap(); // Handle delivery outcome
//!
//!     // Send a message with batchable field set to true
//!     let fut = sender.send_batchable("hello batchable AMQP").await.unwrap();
//!     let outcome: Outcome = fut.await.unwrap(); // Wait for outcome (Disposition)
//!     outcome.accepted_or_else(|state| state).unwrap(); // Handle delivery outcome
//!
//!     // Receive the message from the broker
//!     let delivery = receiver.recv::<String>().await.unwrap();
//!     receiver.accept(&delivery).await.unwrap();
//!
//!     sender.close().await.unwrap(); // Detach sender with closing Detach performatives
//!     receiver.close().await.unwrap(); // Detach receiver with closing Detach performatives
//!     session.end().await.unwrap(); // End the session
//!     connection.close().await.unwrap(); // Close the connection
//! }
//! ```
//!
//! ## Listener
//!
//! ```rust
//! use tokio::net::TcpListener;
//! use fe2o3_amqp::acceptor::{ConnectionAcceptor, SessionAcceptor, LinkAcceptor, LinkEndpoint};
//!
//! #[tokio::main]
//! async fn main() {
//!     let tcp_listener = TcpListener::bind("localhost:5672").await.unwrap();
//!     let connection_acceptor = ConnectionAcceptor::new("example-listener");
//!
//!     while let Ok((stream, addr)) = tcp_listener.accept().await {
//!         let mut connection = connection_acceptor.accept(stream).await.unwrap();
//!         let handle = tokio::spawn(async move {
//!             let session_acceptor = SessionAcceptor::new();
//!             while let Ok(mut session) = session_acceptor.accept(&mut connection).await{
//!                 let handle = tokio::spawn(async move {
//!                     let link_acceptor = LinkAcceptor::new();
//!                     match link_acceptor.accept(&mut session).await.unwrap() {
//!                         LinkEndpoint::Sender(sender) => { },
//!                         LinkEndpoint::Receiver(recver) => { },
//!                     }
//!                 });
//!             }
//!         });
//!     }
//! }
//! ```
//!
//! ## WebSocket
//!
//! [`fe2o3-amqp-ws`](https://crates.io/crates/fe2o3-amqp-ws) is needed for WebSocket binding
//!
//! ```rust
//! use fe2o3_amqp::{
//!     types::{messaging::Outcome, primitives::Value},
//!     Connection, Delivery, Receiver, Sender, Session,
//! };
//! use fe2o3_amqp_ws::WebSocketStream;
//!
//! #[tokio::main]
//! async fn main() {
//!     let (ws_stream, _response) = WebSocketStream::connect("ws://localhost:5673")
//!         .await
//!         .unwrap();
//!     let mut connection = Connection::builder()
//!         .container_id("connection-1")
//!         .open_with_stream(ws_stream)
//!         .await
//!         .unwrap();
//!
//!     connection.close().await.unwrap();
//! }
//! ```
//!
//! # More examples
//!
//! More examples of sending and receiving can be found on the [GitHub repo](https://github.com/minghuaw/fe2o3-amqp/tree/main/examples/).
//! Please note that most examples requires a local broker running. One broker that can be used on Windows is [TestAmqpBroker](https://azure.github.io/amqpnetlite/articles/hello_amqp.html).
//!
//! # Components
//!
//! | Name | Description |
//! |------|-------------|
//! |`serde_amqp_derive`| Custom derive macro for described types as defined in AMQP1.0 protocol |
//! |`serde_amqp`| AMQP1.0 serializer and deserializer as well as primitive types |
//! |`fe2o3-amqp-types`| AMQP1.0 data types |
//! |`fe2o3-amqp`| Implementation of AMQP1.0 `Connection`, `Session`, and `Link` |
//! |`fe2o3-amqp-ext`| Extension types and implementations |
//! |`fe2o3-amqp-ws` | WebSocket binding for `fe2o3-amqp` transport |
//! |`fe2o3-amqp-management`| Experimental implementation of AMQP1.0 management |
//! |`fe2o3-amqp-cbs`| Experimental implementation of AMQP1.0 CBS |
//!
//! # Minimum rust version supported
//!
//! 1.56.0 (ie. 2021 edition)

pub(crate) mod control;
pub(crate) mod endpoint;
pub(crate) mod util;

pub mod auth;
pub mod connection;
pub mod frames;
pub mod link;
pub mod sasl_profile;
pub mod session;
pub mod transport;

#[cfg_attr(docsrs, doc(cfg(feature = "acceptor")))]
#[cfg(feature = "acceptor")]
pub mod acceptor;

#[cfg_attr(docsrs, doc(cfg(feature = "transaction")))]
#[cfg(feature = "transaction")]
pub mod transaction;

pub mod types {
    //! Re-exporting `fe2o3-amqp-types`
    pub use fe2o3_amqp_types::*;
}

pub use connection::Connection;
pub use link::{
    delivery::{Delivery, Sendable},
    Receiver, Sender,
};
pub use session::Session;

type Payload = bytes::Bytes;
