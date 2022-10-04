use client::CbsClient;
use error::CbsClientError;
use fe2o3_amqp::connection::{self, ConnectionHandle, OpenError};
use fe2o3_amqp_management::client::MgmtClient;

pub mod client;
pub mod constants;
pub mod error;

pub async fn open_connection_and_perform_cbs<'a, Mode, Tls>(
    builder: connection::Builder<'a, Mode, Tls>,
) -> Result<ConnectionHandle<()>, CbsClientError> {
    todo!()
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
