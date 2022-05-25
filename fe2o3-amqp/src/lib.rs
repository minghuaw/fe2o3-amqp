#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs, missing_debug_implementations)]

//! A rust implementation of ASMQP 1.0 protocol based on serde and tokio.
//!
//! [![crate_version](https://img.shields.io/crates/v/fe2o3-amqp.svg?style=flat)](https://crates.io/crates/fe2o3-amqp) [![docs_version](https://img.shields.io/badge/docs-latest-blue.svg?style=flat)](https://docs.rs/fe2o3-amqp/latest/fe2o3_amqp/)
//!
//! # Feature flags
//!
//! default: `[]`
//!
//! - `"rustls"`: enables TLS integration with `tokio-rustls` and `rustls`
//! - `"native-tls"`: enables TLS integration with `tokio-native-tls` and `native-tls`
//! - `"acceptor"`: enables `ConnectionAcceptor`, `SessionAcceptor`, and `LinkAcceptor`
//! - `"transaction"`: enables `Controller`, `Transaction`, `TxnAcquisition`
//! (only the client side is implemented so far)
//!
//! # Quick start
//!
//! 1. [Client](#client)
//! 2. [Listener](#listener)
//!
//! ## Client
//!
//! Below is an example with a local broker (
//! [`TestAmqpBroker`](https://github.com/Azure/amqpnetlite/releases/download/test_broker.1609/TestAmqpBroker.zip))
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
//!     // Send a message to the broker
//!     sender.send("hello AMQP").await.unwrap();
//!     
//!     // Receive the message from the broker
//!     let delivery = receiver.recv::<String>().await.unwrap();
//!     receiver.accept(&delivery).await.unwrap();
//!
//!     // Detach links with closing Detach performatives
//!     sender.close().await.unwrap();
//!     receiver.close().await.unwrap();
//!
//!     // End the session
//!     session.end().await.unwrap();
//!
//!     // Close the connection
//!     connection.close().await.unwrap();
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
//! # More examples
//!
//! More examples of sending and receiving can be found on the [GitHub repo](https://github.com/minghuaw/fe2o3-amqp/tree/main/examples/protocol_test).
//! The example has been used for testing with a local [TestAmqpBroker](https://azure.github.io/amqpnetlite/articles/hello_amqp.html).

pub(crate) mod control;
pub(crate) mod util;

pub mod connection;
pub mod endpoint;
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
