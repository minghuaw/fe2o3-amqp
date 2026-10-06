use std::sync::{Arc, OnceLock};

use fe2o3_amqp_types::{definitions, performatives::Detach};
use tokio::sync::mpsc;

use crate::{
    control::SessionControl,
    endpoint::{self, LinkAttach, LinkDetach, LinkExt},
    session::{self, error::AllocLinkError},
};

use super::{
    link_error_from_closed_channel, state::LinkState, AttachMode, DetachError, LinkFrame,
    LinkOutcome, LinkRelay, LinkStateError, SessionStopped,
};

pub(crate) trait LinkEndpointInner
where
    Self: Send + Sync,
    <Self::Link as LinkAttach>::AttachError: From<AllocLinkError> + Send + Sync,
{
    type Link: endpoint::LinkExt + Send + Sync;

    fn link(&self) -> &Self::Link;

    fn link_mut(&mut self) -> &mut Self::Link;

    fn reader_mut(&mut self) -> &mut mpsc::Receiver<LinkFrame>;

    /// The link endpoint's local state, used to derive state-aware failures
    fn local_state(&self) -> &LinkState;

    fn buffer_size(&self) -> usize;

    fn as_new_link_relay(&self, tx: mpsc::Sender<LinkFrame>) -> LinkRelay<()>;

    fn session_control(&self) -> &mpsc::Sender<SessionControl>;

    /// The shared cell holding why the session (or its connection) stopped
    fn session_stop_reason(&self) -> &Arc<OnceLock<SessionStopped>>;

    async fn exchange_attach(
        &mut self,
        mode: AttachMode,
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
        match self.exchange_attach(AttachMode::Reattach).await // cancel safe: the attach exchange only awaits on mpsc operations
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
    ) -> Result<LinkOutcome, <Self::Link as LinkDetach>::DetachError>;

    /// Close the link.
    ///
    /// This will send a `Detach` performative with the `closed` field set to true.
    async fn close_with_error(
        &mut self,
        error: Option<definitions::Error>,
    ) -> Result<LinkOutcome, <Self::Link as LinkDetach>::DetachError>;
}

/// Link endpoints that must terminate the link when a remote-initiated
/// transactional acquisition arrives, since it is not supported.
#[cfg(feature = "transaction")]
pub(crate) trait TxnAcquisitionCloseExt: LinkEndpointInnerDetach {
    /// Best-effort terminate the link with `amqp:not-implemented` because a
    /// remote-initiated transactional acquisition is not supported
    /// (AMQP 1.0 §4.4.3). The caller reports the acquisition error.
    async fn close_on_acquisition_not_implemented(&mut self);
}

#[cfg(feature = "transaction")]
impl<T> TxnAcquisitionCloseExt for T
where
    T: LinkEndpointInnerDetach,
{
    async fn close_on_acquisition_not_implemented(&mut self) {
        let error = definitions::Error::new(
            definitions::AmqpError::NotImplemented,
            "Transactional acquisition is not implemented".to_string(),
            None,
        );
        let _ = self.close_with_error(Some(error)).await;
    }
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
    ) -> Result<LinkOutcome, <Self::Link as LinkDetach>::DetachError> {
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
                    // AMQP 1.0 §2.6.6: the peer sent a closing detach while we
                    // were sending a non-closing detach, so we must reattach
                    // and then send a closing detach. The link ends `Closed`;
                    // the peer's error, if any, is reported in the status.
                    let status = LinkOutcome::Closed {
                        remote_error: remote_detach.error.clone(),
                    };
                    reattach_then_close(self).await?;
                    Ok(status)
                } else {
                    self.link_mut().on_detach_reply(remote_detach)
                }
            }
            LinkState::DetachSent => {
                let remote_detach = recv_remote_detach(self).await?;
                if remote_detach.closed {
                    let status = LinkOutcome::Closed {
                        remote_error: remote_detach.error.clone(),
                    };
                    reattach_then_close(self).await?;
                    Ok(status)
                } else {
                    self.link_mut().on_detach_reply(remote_detach)
                }
            }
            LinkState::Detached(remote_error) => Ok(LinkOutcome::Detached {
                remote_error: remote_error.clone(),
            }),
            LinkState::CloseSent => {
                // A live handle is not normally left in `CloseSent` (the
                // public close paths consume it, and dropping the close
                // future drops it too). It can happen when
                // `reattach_then_close` is cancelled after sending the
                // closing detach but before its reply arrives, while the
                // session is still alive; the handle survives because
                // `detach_then_resume_on_session` takes `&mut self`. (If the
                // session stopped instead, `recv_remote_detach` fails
                // immediately and there is no reply to consume.)
                #[cfg(feature = "tracing")]
                tracing::warn!(
                    "detach_with_error called on a link in CloseSent; completing the closing handshake"
                );
                #[cfg(feature = "log")]
                log::warn!(
                    "detach_with_error called on a link in CloseSent; completing the closing handshake"
                );

                // The link is already closing, so a detach cannot suspend
                // it: consume the pending reply, if any, and complete the
                // closing handshake.
                let remote_detach = recv_remote_detach(self).await?;
                if remote_detach.closed {
                    self.link_mut().on_detach_reply(remote_detach)
                } else {
                    // The peer suspended: reattach and close so the link is left
                    // `Closed` (AMQP 1.0 §2.6.6).
                    let status = LinkOutcome::Closed {
                        remote_error: remote_detach.error.clone(),
                    };
                    let _ = self.link_mut().apply_remote_detach_outcome(remote_detach);
                    reattach_then_close(self).await?;
                    Ok(status)
                }
            }
            LinkState::Closed(remote_error) => Ok(LinkOutcome::Closed {
                remote_error: remote_error.clone(),
            }),
        }
    }

    /// # Cancel safety
    ///
    /// This should be cancel safe if oneshot channel is cancel safe
    async fn close_with_error(
        &mut self,
        error: Option<definitions::Error>,
    ) -> Result<LinkOutcome, <Self::Link as LinkDetach>::DetachError> {
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
                self.send_detach(true, error).await?; // cancel safe

                // Wait for remote detach
                let remote_detach = recv_remote_detach(self).await?; // cancel safe
                if remote_detach.closed {
                    // The peer's error, if any, is surfaced in the returned status
                    self.link_mut().on_detach_reply(remote_detach)
                } else {
                    // Peer suspended while we were closing: record it, then
                    // reattach (re-registers the link) and close (§2.6.6).
                    // The close completes, so this is not an error.
                    let status = LinkOutcome::Closed {
                        remote_error: remote_detach.error.clone(),
                    };
                    let _ = self.link_mut().apply_remote_detach_outcome(remote_detach);
                    reattach_then_close(self).await?;
                    Ok(status)
                }
            }
            LinkState::DetachSent => {
                // We already sent a non-closing detach and `close()` is now
                // called. Wait for the reply.
                let remote_detach = recv_remote_detach(self).await?; // cancel safe
                if remote_detach.closed {
                    // §2.6.6: reattach and then send a closing detach.
                    let status = LinkOutcome::Closed {
                        remote_error: remote_detach.error.clone(),
                    };
                    reattach_then_close(self).await?;
                    Ok(status)
                } else {
                    let status = LinkOutcome::Closed {
                        remote_error: remote_detach.error.clone(),
                    };
                    let _ = self.link_mut().apply_remote_detach_outcome(remote_detach);
                    reattach_then_close(self).await?;
                    Ok(status)
                }
            }
            LinkState::Detached(remote_error) => Ok(LinkOutcome::Detached {
                remote_error: remote_error.clone(),
            }),
            LinkState::CloseSent => {
                // Wait for remote detach
                let remote_detach = recv_remote_detach(self).await?; // cancel safe
                if remote_detach.closed {
                    self.link_mut().on_detach_reply(remote_detach)
                } else {
                    // Peer suspended while we were closing: reattach
                    // (re-registers the link) and close (§2.6.6).
                    // The close completes, so this is not an error.
                    let status = LinkOutcome::Closed {
                        remote_error: remote_detach.error.clone(),
                    };
                    let _ = self.link_mut().apply_remote_detach_outcome(remote_detach);
                    reattach_then_close(self).await?;
                    Ok(status)
                }
            }
            LinkState::Closed(remote_error) => Ok(LinkOutcome::Closed {
                remote_error: remote_error.clone(),
            }),
        }
    }
}

/// AMQP 1.0 §2.6.6: when one peer sends a closing detach while its partner
/// is sending a non-closing detach, the partner MUST signal that it has
/// closed the link by reattaching and then sending a closing detach.
///
/// Used on both sides of the race. The closing side reattaches too so the
/// link is re-registered for the peer's crossed attach (it is released when
/// the closing detach is sent); the crossed attach exchanges then converge
/// symmetrically.
///
/// # Cancel safety
///
/// This is cancel safe if oneshot channel is cancel safe
async fn reattach_then_close<T>(link_inner: &mut T) -> Result<(), DetachError>
where
    T: LinkEndpointInner + LinkEndpointInnerReattach + Send + Sync,
    T::Link: LinkDetach<DetachError = DetachError>,
    <T::Link as LinkAttach>::AttachError: From<AllocLinkError> + Sync,
{
    if let Err(_attach_error) = link_inner.reattach_inner().await {
        // The reattach that completes the AMQP 1.0 §2.6.6 handshake failed.
        // This helper is generic over the link type, so the concrete
        // `AttachError` cannot be inspected here. The failure is derived from
        // the session stop reason and the link's local state instead.
        //
        // TODO(error-refactor): preserve the concrete attach error so these
        // cases become fully distinguishable from a terminal link.
        return Err(link_error_from_closed_channel(
            link_inner.session_stop_reason(),
            link_inner.local_state(),
        ));
    }
    link_inner.send_detach(true, None).await?; // cancel safe
    let remote_detach = recv_remote_detach(link_inner).await?; // cancel safe
    link_inner.link_mut().on_detach_reply(remote_detach)?;
    Ok(())
}

/// # Cancel safety
///
/// This is cancel safe because it only `.await` on `recv()` from a `tokio::mpsc::Receiver`
pub(super) async fn recv_remote_detach<T>(link_inner: &mut T) -> Result<Detach, LinkStateError>
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
            .ok_or_else(|| {
                link_error_from_closed_channel(
                    link_inner.session_stop_reason(),
                    link_inner.local_state(),
                )
            })? {
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
