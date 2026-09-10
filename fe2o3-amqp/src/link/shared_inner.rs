use std::sync::{Arc, OnceLock};

use fe2o3_amqp_types::{definitions, performatives::Detach};
use tokio::sync::mpsc;

use crate::{
    control::SessionControl,
    endpoint::{self, LinkAttach, LinkDetach, LinkExt},
    session::{self, error::AllocLinkError},
};

use super::{state::LinkState, DetachError, LinkFrame, LinkRelay, SessionStopReason};

pub(crate) trait LinkEndpointInner
where
    Self: Send + Sync,
    <Self::Link as LinkAttach>::AttachError: From<AllocLinkError> + Send + Sync,
{
    type Link: endpoint::LinkExt + Send + Sync;

    fn link(&self) -> &Self::Link;

    fn link_mut(&mut self) -> &mut Self::Link;

    fn reader_mut(&mut self) -> &mut mpsc::Receiver<LinkFrame>;

    fn buffer_size(&self) -> usize;

    fn as_new_link_relay(&self, tx: mpsc::Sender<LinkFrame>) -> LinkRelay<()>;

    fn session_control(&self) -> &mpsc::Sender<SessionControl>;

    /// The shared cell holding why the session (or its connection) stopped
    fn session_stop_reason(&self) -> &Arc<OnceLock<SessionStopReason>>;

    async fn exchange_attach(
        &mut self,
        is_reattaching: bool,
    ) -> Result<<Self::Link as LinkAttach>::AttachExchange, <Self::Link as LinkAttach>::AttachError>;

    async fn handle_attach_error(
        &mut self,
        attach_error: <Self::Link as LinkAttach>::AttachError,
    ) -> <Self::Link as LinkAttach>::AttachError;

    /// This should be cancel safe because the implementation should be a simple sending on `tokio::mpsc::Sender`
    async fn send_detach(
        &mut self,
        closed: bool,
        error: Option<definitions::Error>,
    ) -> Result<(), <Self::Link as LinkDetach>::DetachError>;

    /// # Cancel safety
    ///
    /// This should be cancel safe if oneshot channel is cancel safe
    async fn reallocate_output_handle(
        &mut self,
    ) -> Result<(), <Self::Link as LinkAttach>::AttachError> {
        let (tx, incoming) = mpsc::channel(self.buffer_size());
        let link_relay = self.as_new_link_relay(tx);
        *self.reader_mut() = incoming;
        let link_name = self.link().name().to_string();
        let handle = session::allocate_link(
            self.session_control(),
            link_name,
            link_relay,
            self.session_stop_reason(),
        )
        .await?; // FIXME: cancel safe?
        *self.link_mut().output_handle_mut() = Some(handle);
        Ok(())
    }
}

pub(crate) trait LinkEndpointInnerReattach
where
    Self: LinkEndpointInner + Send + Sync,
    <Self::Link as LinkAttach>::AttachError: From<AllocLinkError> + Send + Sync,
{
    fn handle_reattach_outcome(
        &mut self,
        outcome: <Self::Link as LinkAttach>::AttachExchange,
    ) -> Result<&mut Self, <Self::Link as LinkAttach>::AttachError>;

    /// # Cancel safety
    ///
    /// This should be cancel safe if oneshot channel is cancel safe
    async fn reattach_inner(
        &mut self,
    ) -> Result<&mut Self, <Self::Link as LinkAttach>::AttachError> {
        self.reallocate_output_handle().await?; // FIXME: cancel safe? if oneshot channel is cancel safe
        match self.exchange_attach(true).await // cancel safe: the attach exchange only awaits on mpsc operations
        {
            Ok(attach_exchange) => self.handle_reattach_outcome(attach_exchange),
            Err(attach_error) => Err(self.handle_attach_error(attach_error).await),
        }
    }
}

pub(crate) trait LinkEndpointInnerDetach
where
    Self: LinkEndpointInner,
    <Self::Link as LinkAttach>::AttachError: From<AllocLinkError> + Send + Sync,
{
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

impl<T> LinkEndpointInnerDetach for T
where
    T: LinkEndpointInner + LinkEndpointInnerReattach + Send + Sync,
    T::Link: LinkDetach<DetachError = DetachError>,
    <T::Link as LinkAttach>::AttachError: From<AllocLinkError> + Sync,
{
    async fn detach_with_error(
        &mut self,
        error: Option<definitions::Error>,
    ) -> Result<(), <Self::Link as LinkDetach>::DetachError> {
        match self.link().local_state() {
            LinkState::Unattached
            | LinkState::AttachSent
            | LinkState::IncompleteAttachSent
            | LinkState::IncompleteAttachReceived
            | LinkState::IncompleteAttachExchanged
            | LinkState::AttachReceived
            | LinkState::Attached => {
                // Send a non-closing detach
                self.send_detach(false, error).await?;

                let remote_detach = recv_remote_detach(self).await?;
                if remote_detach.closed {
                    // AMQP 1.0 §2.6.6: one peer MAY send a closing detach while
                    // its partner is sending a non-closing detach. In this case
                    // the partner (us) MUST signal that it has closed the link
                    // by reattaching and then sending a closing detach.
                    self.reattach_inner()
                        .await
                        .map_err(|_| DetachError::DetachedByRemote)?;
                    self.send_detach(true, None).await?;
                    let remote_detach = recv_remote_detach(self).await?;
                    self.link_mut().on_incoming_detach(remote_detach)?;
                    Err(DetachError::ClosedByRemote)
                } else {
                    self.link_mut().on_incoming_detach(remote_detach)
                }
            }
            LinkState::DetachSent => {
                let remote_detach = recv_remote_detach(self).await?;
                if remote_detach.closed {
                    // §2.6.6: reattach and then send a closing detach.
                    self.reattach_inner()
                        .await
                        .map_err(|_| DetachError::DetachedByRemote)?;
                    self.send_detach(true, None).await?;
                    let remote_detach = recv_remote_detach(self).await?;
                    self.link_mut().on_incoming_detach(remote_detach)?;
                    Err(DetachError::ClosedByRemote)
                } else {
                    self.link_mut().on_incoming_detach(remote_detach)
                }
            }
            LinkState::Detached => Ok(()),
            LinkState::CloseSent => {
                let remote_detach = recv_remote_detach(self).await?;
                let _ = self.link_mut().apply_remote_detach_outcome(remote_detach);
                Err(DetachError::ClosedByRemote)
            }
            LinkState::Closed => Err(DetachError::ClosedByRemote),
        }
    }

    /// # Cancel safety
    ///
    /// This should be cancel safe if oneshot channel is cancel safe
    async fn close_with_error(
        &mut self,
        error: Option<definitions::Error>,
    ) -> Result<(), <Self::Link as LinkDetach>::DetachError> {
        match self.link().local_state() {
            LinkState::Unattached
            | LinkState::AttachSent
            | LinkState::IncompleteAttachSent
            | LinkState::IncompleteAttachReceived
            | LinkState::IncompleteAttachExchanged
            | LinkState::AttachReceived
            | LinkState::Attached => {
                // Send detach with closed=true and wait for remote closing detach
                // The sender will be dropped after close
                self.send_detach(true, error)
                    .await // cancel safe
                    .map_err(|_| detach_error_from_stop_reason(self))?;

                // Wait for remote detach
                let remote_detach = recv_remote_detach(self).await?; // cancel safe
                if remote_detach.closed {
                    // If the remote detach contains an error, the error will be propagated
                    // back by `on_incoming_detach`
                    self.link_mut().on_incoming_detach(remote_detach)
                } else {
                    // The peer suspended (non-closing detach) while we were
                    // closing. Per AMQP 1.0 §2.6.6 the non-closing side (the
                    // peer) reattaches and then closes; this side records the
                    // detach outcome and does not reattach itself.
                    match self.link_mut().apply_remote_detach_outcome(remote_detach) {
                        Ok(()) => Err(DetachError::DetachedByRemote),
                        Err(error) => Err(error),
                    }
                }
            }
            LinkState::DetachSent => {
                // We already sent a non-closing detach and `close()` is now
                // called. Record whatever the peer answered (Closed or
                // Detached); this side does not reattach.
                let remote_detach = recv_remote_detach(self).await?; // cancel safe
                self.link_mut().apply_remote_detach_outcome(remote_detach)
            }
            LinkState::Detached => Ok(()),
            LinkState::CloseSent => {
                // Wait for remote detach
                let remote_detach = recv_remote_detach(self).await?; // cancel safe
                if remote_detach.closed {
                    self.link_mut().on_incoming_detach(remote_detach)
                } else {
                    match self.link_mut().apply_remote_detach_outcome(remote_detach) {
                        Ok(()) => Err(DetachError::DetachedByRemote),
                        Err(error) => Err(error),
                    }
                }
            }
            LinkState::Closed => Ok(()),
        }
    }
}

/// The `DetachError` for a link operation that failed because the session (or its
/// connection) stopped; `IllegalState` when no stop reason was recorded (defensive).
fn detach_error_from_stop_reason<T>(inner: &T) -> DetachError
where
    T: LinkEndpointInner + ?Sized,
{
    match inner.session_stop_reason().get() {
        Some(reason) => DetachError::SessionStopped(reason.clone()),
        None => DetachError::IllegalState, // defensive: no stop reason recorded; failure is link-local
    }
}

/// # Cancel safety
///
/// This is cancel safe because it only `.await` on `recv()` from a `tokio::mpsc::Receiver`
pub(super) async fn recv_remote_detach<T>(link_inner: &mut T) -> Result<Detach, DetachError>
where
    T: LinkEndpointInner + LinkEndpointInnerReattach + Send + Sync,
    T::Link: LinkDetach<DetachError = DetachError>,
    <T::Link as LinkAttach>::AttachError: From<AllocLinkError> + Sync,
{
    loop {
        match link_inner
            .reader_mut()
            .recv()
            .await // cancel safe
            .ok_or_else(|| detach_error_from_stop_reason(link_inner))?
        {
            LinkFrame::Detach(detach) => return Ok(detach),
            _frame => {
                // The only other frames should be Attach or Detach, (or Transfer if receiver).
                // Ignore all other frames
                #[cfg(feature = "tracing")]
                tracing::debug!("Non-detach frame received: {:?}", _frame);
                #[cfg(feature = "log")]
                log::debug!("Non-detach frame received: {:?}", _frame);
                continue;
            }
        }
    }
}
