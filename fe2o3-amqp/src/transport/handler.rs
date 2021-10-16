//! Trait definition for handler APIs
//! TODO: consider moving and restricting this to only the corresponding
//! listener types (eg. ListenerConnection etc)

use async_trait::async_trait;
use fe2o3_amqp_types::performatives::Open;

use crate::error::EngineError;
use super::endpoint;

/// The handler when an Open frame is received from the remote peer
#[async_trait]
pub trait OnOpen {
    async fn on_open(connection: &mut dyn endpoint::Connection<Error=EngineError>, remote_open: &Open);
}

/// The handler when a Begin frame is received from the remote peer
#[async_trait]
pub trait OnBegin {

}