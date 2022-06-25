use async_trait::async_trait;
use fe2o3_amqp_types::definitions;
use tokio::sync::mpsc;

use crate::util::Consumer;
use crate::{
    control::SessionControl,
    endpoint::{self, LinkAttach, LinkDetach, LinkExt},
    session::{self, AllocLinkError},
};

use super::{LinkFrame, LinkRelay};

#[async_trait]
pub(crate) trait LinkEndpointInner {
    type Link: endpoint::LinkExt + Send + Sync;

    fn link(&self) -> &Self::Link;

    fn link_mut(&mut self) -> &mut Self::Link;

    fn writer(&self) -> &mpsc::Sender<LinkFrame>;

    fn reader_mut(&mut self) -> &mut mpsc::Receiver<LinkFrame>;

    fn buffer_size(&self) -> usize;

    fn as_new_link_relay(&self, tx: mpsc::Sender<LinkFrame>) -> LinkRelay<()>;

    fn session_control(&self) -> &mpsc::Sender<SessionControl>;

    async fn negotiate_attach(&mut self) -> Result<(), <Self::Link as LinkAttach>::AttachError>;

    async fn handle_attach_error(
        &mut self,
        attach_error: <Self::Link as LinkAttach>::AttachError,
    ) -> <Self::Link as LinkAttach>::AttachError;
}

#[async_trait]
pub(crate) trait LinkEndpointInnerDetach: LinkEndpointInner {
    async fn close_with_error(
        &mut self,
        error: Option<definitions::Error>,
    ) -> Result<(), <Self::Link as LinkDetach>::DetachError>;

    async fn detach_with_error(
        &mut self,
        error: Option<definitions::Error>,
    ) -> Result<(), <Self::Link as LinkDetach>::DetachError>;
}

#[async_trait]
pub(crate) trait LinkEndpointInnerReattach {
    type ReattachError: Send + From<AllocLinkError>;

    async fn reattach_inner(
        &mut self,
    ) -> Result<&mut Self, Self::ReattachError>;
}

#[async_trait]
impl<T> LinkEndpointInnerReattach for T
where
    T: LinkEndpointInner + Send + Sync,
    <T::Link as LinkAttach>::AttachError: From<AllocLinkError>,
{
    type ReattachError = <T::Link as LinkAttach>::AttachError;

    async fn reattach_inner(
        &mut self,
    ) -> Result<&mut Self, Self::ReattachError> {
        if self.link().output_handle().is_none() {
            let (tx, incoming) = mpsc::channel(self.buffer_size());
            let link_relay = self.as_new_link_relay(tx);
            *self.reader_mut() = incoming;
            let link_name = self.link().name().to_string();
            let handle = session::allocate_link(self.session_control(), link_name, link_relay).await?;
            *self.link_mut().output_handle_mut() = Some(handle);
        }

        match self.negotiate_attach().await {
            Ok(_) => Ok(self),
            Err(attach_error) => Err(self
                .handle_attach_error(attach_error)
                .await),
        }
    }
}
