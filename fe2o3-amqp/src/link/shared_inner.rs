use async_trait::async_trait;
use fe2o3_amqp_types::{definitions, performatives::Detach};
use tokio::sync::mpsc;

use crate::{
    control::SessionControl,
    endpoint::{self, LinkAttach, LinkDetach, LinkExt},
    session::{self, AllocLinkError},
};

use super::{state::LinkState, DetachError, LinkFrame, LinkRelay};

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

    async fn send_detach(
        &mut self,
        closed: bool,
        error: Option<definitions::Error>,
    ) -> Result<(), <Self::Link as LinkDetach>::DetachError>;
}

#[async_trait]
pub(crate) trait LinkEndpointInnerReattach {
    type ReattachError: Send + From<AllocLinkError>;

    async fn reattach_inner(&mut self) -> Result<&mut Self, Self::ReattachError>;
}

#[async_trait]
impl<T> LinkEndpointInnerReattach for T
where
    T: LinkEndpointInner + Send + Sync,
    <T::Link as LinkAttach>::AttachError: From<AllocLinkError>,
{
    type ReattachError = <T::Link as LinkAttach>::AttachError;

    async fn reattach_inner(&mut self) -> Result<&mut Self, Self::ReattachError> {
        if self.link().output_handle().is_none() {
            let (tx, incoming) = mpsc::channel(self.buffer_size());
            let link_relay = self.as_new_link_relay(tx);
            *self.reader_mut() = incoming;
            let link_name = self.link().name().to_string();
            let handle =
                session::allocate_link(self.session_control(), link_name, link_relay).await?;
            *self.link_mut().output_handle_mut() = Some(handle);
        }

        match self.negotiate_attach().await {
            Ok(_) => Ok(self),
            Err(attach_error) => Err(self.handle_attach_error(attach_error).await),
        }
    }
}

#[async_trait]
pub(crate) trait LinkEndpointInnerDetach: LinkEndpointInner {
    /// Detach the link.
    ///
    /// This will send a `Detach` performative with the `closed` field set to false. If the remote
    /// peer responds with a Detach performative whose `closed` field is set to true, the link will
    /// re-attach and then close by exchanging closing Detach performatives.
    async fn detach_with_error(
        &mut self,
        error: Option<definitions::Error>,
    ) -> Result<(), <Self::Link as LinkDetach>::DetachError>;

    /// Close the link.
    ///
    /// This will send a Detach performative with the `closed` field set to true.
    async fn close_with_error(
        &mut self,
        error: Option<definitions::Error>,
    ) -> Result<(), <Self::Link as LinkDetach>::DetachError>;
}

#[async_trait]
impl<T> LinkEndpointInnerDetach for T
where
    T: LinkEndpointInner + LinkEndpointInnerReattach + Send + Sync,
    T::Link: LinkDetach<DetachError = DetachError>,
{
    async fn detach_with_error(
        &mut self,
        error: Option<definitions::Error>,
    ) -> Result<(), <Self::Link as LinkDetach>::DetachError> {
        match self.link().local_state() {
            LinkState::Unattached
            | LinkState::AttachSent
            | LinkState::AttachReceived
            | LinkState::Attached => {
                // Send a non-closing detach
                self.send_detach(false, error).await?;

                let remote_detach = recv_remote_detach(self).await?;
                if remote_detach.closed {
                    // Note that one peer MAY send a closing detach while its partner is
                    // sending a non-closing detach. In this case, the partner MUST
                    // signal that it has closed the link by reattaching and then sending
                    // a closing detach.
                    reattach_and_then_close(self).await?;

                    // A peer closes a link by sending the detach frame with the handle for the
                    // specified link, and the closed flag set to true. The partner will destroy
                    // the corresponding link endpoint, and reply with its own detach frame with
                    // the closed flag set to true.
                    Err(DetachError::ClosedByRemote)
                } else {
                    self.link_mut().on_incoming_detach(remote_detach).await
                }
            }
            LinkState::DetachSent => {
                let remote_detach = recv_remote_detach(self).await?;
                if remote_detach.closed {
                    reattach_and_then_close(self).await?;
                    Err(DetachError::ClosedByRemote)
                } else {
                    self.link_mut().on_incoming_detach(remote_detach).await
                }
            }
            LinkState::DetachReceived => self.send_detach(false, error).await,
            LinkState::Detached => Ok(()),
            LinkState::CloseSent => {
                // This should be impossible.
                // FIXME: treat it as if remote closed
                let _remote_detach = recv_remote_detach(self).await?;
                reattach_and_then_close(self).await?;
                Err(DetachError::ClosedByRemote)
            }
            LinkState::CloseReceived => {
                self.send_detach(true, error).await?;
                Err(DetachError::ClosedByRemote)
            }
            LinkState::Closed => Err(DetachError::ClosedByRemote),
        }
    }

    async fn close_with_error(
        &mut self,
        error: Option<definitions::Error>,
    ) -> Result<(), <Self::Link as LinkDetach>::DetachError> {
        match self.link().local_state() {
            LinkState::Unattached
            | LinkState::AttachSent
            | LinkState::AttachReceived
            | LinkState::Attached => {
                // Send detach with closed=true and wait for remote closing detach
                // The sender will be dropped after close
                self.send_detach(true, error)
                    .await
                    .map_err(|_| DetachError::IllegalSessionState)?;

                // Wait for remote detach
                let remote_detach = recv_remote_detach(self).await?;
                if remote_detach.closed {
                    // If the remote detach contains an error, the error will be propagated
                    // back by `on_incoming_detach`
                    self.link_mut().on_incoming_detach(remote_detach).await
                } else {
                    reattach_and_then_close(self).await
                }
            }
            LinkState::DetachSent => {
                // FIXME: this should be impossible
                // Wait for remote detach
                let _remote_detach = recv_remote_detach(self).await?;
                reattach_and_then_close(self).await?;
                Err(DetachError::DetachedByRemote)
            }
            LinkState::DetachReceived => self
                .send_detach(true, error)
                .await
                .map_err(|_| DetachError::IllegalSessionState),
            LinkState::Detached => reattach_and_then_close(self).await,
            LinkState::CloseSent => {
                // Wait for remote detach
                let remote_detach = recv_remote_detach(self).await?;
                if remote_detach.closed {
                    self.link_mut().on_incoming_detach(remote_detach).await
                } else {
                    reattach_and_then_close(self).await
                }
            }
            LinkState::CloseReceived => self
                .send_detach(true, error)
                .await
                .map_err(|_| DetachError::IllegalSessionState),
            LinkState::Closed => Ok(()),
        }
    }
}

async fn reattach_and_then_close<T>(link_inner: &mut T) -> Result<(), DetachError>
where
    T: LinkEndpointInner + LinkEndpointInnerReattach + Send + Sync,
    T::Link: LinkDetach<DetachError = DetachError>,
{
    // Note that one peer MAY send a closing detach while its partner is
    // sending a non-closing detach. In this case, the partner MUST
    // signal that it has closed the link by reattaching and then sending
    // a closing detach.

    // Probably something like below
    // 1. wait for incoming attach
    // 2. send back attach
    // 3. wait for incoming closing detach
    // 4. detach

    link_inner
        .reattach_inner()
        .await
        .map_err(|_| DetachError::DetachedByRemote)?;
    link_inner.send_detach(true, None).await?;
    let remote_detach = recv_remote_detach(link_inner).await?;
    link_inner
        .link_mut()
        .on_incoming_detach(remote_detach)
        .await?;
    Ok(())
}

async fn recv_remote_detach<T>(link_inner: &mut T) -> Result<Detach, DetachError>
where
    T: LinkEndpointInner + LinkEndpointInnerReattach + Send + Sync,
    T::Link: LinkDetach<DetachError = DetachError>,
{
    match link_inner
        .reader_mut()
        .recv()
        .await
        .ok_or(DetachError::IllegalSessionState)?
    {
        LinkFrame::Detach(detach) => Ok(detach),
        _ => Err(DetachError::NonDetachFrameReceived),
    }
}
