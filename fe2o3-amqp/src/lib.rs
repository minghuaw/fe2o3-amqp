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

pub mod connection;
pub mod control;
pub mod endpoint;
pub mod frames;
pub mod link;
pub mod sasl_profile;
pub mod session;
pub mod transport;
pub mod util;

pub mod types {
    pub use fe2o3_amqp_types::*;
}

type Payload = bytes::Bytes;
