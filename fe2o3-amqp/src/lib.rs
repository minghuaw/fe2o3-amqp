#![warn(missing_docs)]

//! An implementation of ASMQP 1.0 protocol based on serde and tokio.
//!
//! # Quick start
//!
//! TODO
//!
//! # Examples
//!
//! Examples of sending and receiving can be found on the [GitHub repo](https://github.com/minghuaw/fe2o3-amqp/tree/main/examples/protocol_test).
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

pub mod types {
    //! Re-exporting `fe2o3-amqp-types`
    pub use fe2o3_amqp_types::*;
}

pub use connection::{Connection};
pub use session::{Session};
pub use link::{Sender, Receiver};

type Payload = bytes::Bytes;
