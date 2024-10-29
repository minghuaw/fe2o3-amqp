#![deny(missing_docs, missing_debug_implementations)]
#![allow(clippy::result_large_err)] // TODO: refactor in 0.14.0

//! An experimental implementation of AMQP 1.0 management working draft with `fe2o3-amqp`
//!
//! Because the AMQP 1.0 management working draft itself isn't stable yet, this crate is
//! expected to see breaking changes in all future releases until the draft becomes stable.

pub mod client;
pub mod error;
pub mod operations;
pub mod status;

pub mod constants;
pub mod request;
pub mod response;

pub mod mgmt_ext;

/// The default address of the management node.
pub const MANAGEMENT_NODE_ADDRESS: &str = "$management";

/// The default address of the client node.
pub const DEFAULT_CLIENT_NODE_ADDRESS: &str = "mgmt-client";

pub use client::MgmtClient;
pub use request::Request;
pub use response::Response;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
