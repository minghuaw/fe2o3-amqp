use async_trait::async_trait;
use fe2o3_amqp_types::performatives::{Attach, Detach, Disposition, Flow};
use tokio::sync::mpsc;

use crate::{endpoint, error::EngineError};

use super::LinkFrame;




/// Manages the link state
pub struct SenderLink {

}

#[async_trait]
impl endpoint::Link for SenderLink {
    type Error = EngineError;
    
    async fn on_incoming_attach(&mut self, attach: Attach) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_incoming_flow(&mut self, flow: Flow) -> Result<(), Self::Error> {
        todo!()
    }

    // Only the receiver is supposed to receive incoming Transfer frame

    async fn on_incoming_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_incoming_detach(&mut self, detach: Detach) -> Result<(), Self::Error> {
        todo!()
    }

    async fn send_attach(
        &mut self,
        writer: &mut mpsc::Sender<LinkFrame>,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn send_flow(
        &mut self,
        writer: &mut mpsc::Sender<LinkFrame>
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn send_disposition(
        &mut self,
        writer: &mut mpsc::Sender<LinkFrame>
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn send_detach(
        &mut self,
        writer: &mut mpsc::Sender<LinkFrame>,
    ) -> Result<(), Self::Error> {
        todo!()
    }
}

#[async_trait]
impl endpoint::SenderLink for SenderLink {
    async fn send_transfer(&mut self, writer: &mut mpsc::Sender<LinkFrame>) -> Result<(), <Self as endpoint::Link>::Error> {
        todo!()
    }
}
