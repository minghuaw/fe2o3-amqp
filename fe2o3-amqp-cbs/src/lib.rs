use client::CbsClient;
use fe2o3_amqp::connection::{self, ConnectionHandle, OpenError};
use fe2o3_amqp_management::client::MgmtClient;

pub mod client;
pub mod constants;
pub mod error;
pub mod put_token;

/// Open connection with `SaslProfile::Anonymous` and then perform CBS
pub async fn open_connection_and_perform_cbs<'a, Mode, Tls>(
    builder: connection::Builder<'a, Mode, Tls>,
) -> Result<ConnectionHandle<()>, ()> {
    todo!()
}
