use fe2o3_amqp::{link::{SendError, RecvError}, types::messaging::Outcome};

/// Error with CBS client
#[derive(Debug, thiserror::Error)]
pub enum CbsClientError {

}