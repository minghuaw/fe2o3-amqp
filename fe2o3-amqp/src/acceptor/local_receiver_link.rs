//! Implements acceptor for a remote sender link

use std::{
    marker::PhantomData,
    sync::{atomic::AtomicU32, Arc},
};

use fe2o3_amqp_types::{
    messaging::{Target, TargetArchetype},
    performatives::Attach,
    primitives::Symbol,
};
use parking_lot::RwLock;
use tokio::sync::mpsc;

use crate::{
    control::SessionControl,
    endpoint::{InputHandle, LinkAttach, LinkExt},
    link::{
        receiver::{CreditMode, ReceiverInner},
        state::{LinkFlowState, LinkFlowStateInner, LinkState},
        target_archetype::TargetArchetypeExt,
        AttachMode, LinkFrame, LinkIncomingItem, LinkRelay, ReceiverAttachError, ReceiverLink,
    },
    session::SessionHandle,
    Receiver,
};

use super::link::SharedLinkAcceptorFields;

/// An acceptor for a remote Sender link
///
/// the sender is considered to hold the authoritative version of the
/// source properties, the receiver is considered to hold the authoritative version of the target properties.
#[derive(Debug, Clone)]
pub(crate) struct LocalReceiverLinkAcceptor<C, T, F>
where
    F: Fn(T) -> Option<T>,
{
    /// Credit mode of the link. This has no effect on a sender
    pub credit_mode: CreditMode,

    /// the extension capabilities the sender supports/desires
    pub target_capabilities: Option<Vec<C>>,

    /// Whether the receiver will automatically accept all incoming deliveries
    /// 
    /// # Default
    ///
    /// `false`
    pub auto_accept: bool,

    pub on_dynamic_target: F,
    pub target_marker: PhantomData<T>,

    /// Whether the local link will verify the incoming source/target
    pub verify_incoming_source: bool,
    /// Whether the local link will verify the incoming source/target
    pub verify_incoming_target: bool,
}

fn reject_dynamic_target<T>(_: T) -> Option<T> {
    None
}

impl<C, T> Default for LocalReceiverLinkAcceptor<C, T, fn(T) -> Option<T>> {
    fn default() -> Self {
        Self {
            credit_mode: CreditMode::default(),
            target_capabilities: None,
            auto_accept: false,
            on_dynamic_target: reject_dynamic_target,
            target_marker: PhantomData,
            verify_incoming_source: true,
            verify_incoming_target: true,
        }
    }
}

impl<F> LocalReceiverLinkAcceptor<Symbol, Target, F>
where
    F: Fn(Target) -> Option<Target>,
{
    pub async fn accept_incoming_attach<R>(
        &self,
        shared: &SharedLinkAcceptorFields,
        remote_attach: Attach,
        session: &mut SessionHandle<R>,
    ) -> Result<Receiver, ReceiverAttachError> {
        self.accept_incoming_attach_inner(
            shared,
            remote_attach,
            session.control.clone(),
            session.outgoing.clone(),
        )
        .await
        .map(|inner| Receiver { inner })
    }
}

impl<C, T, F> LocalReceiverLinkAcceptor<C, T, F>
where
    C: Clone,
    F: Fn(T) -> Option<T>,
{
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    pub async fn accept_incoming_attach_inner(
        &self,
        shared: &SharedLinkAcceptorFields,
        remote_attach: Attach,
        control: mpsc::Sender<SessionControl>,
        outgoing: mpsc::Sender<LinkFrame>,
    ) -> Result<ReceiverInner<ReceiverLink<T>>, ReceiverAttachError>
    where
        T: Into<TargetArchetype>
            + TryFrom<TargetArchetype>
            + TargetArchetypeExt<Capability = C>
            + Clone
            + Send
            + Sync,
    {
        let snd_settle_mode = if shared
            .supported_snd_settle_modes
            .supports(&remote_attach.snd_settle_mode)
        {
            remote_attach.snd_settle_mode.clone()
        } else {
            shared.fallback_snd_settle_mode.clone()
        };
        // The receiver SHOULD respect the sender’s desired settlement mode if
        // the sender initiates the attach exchange and the receiver supports the desired mode
        let rcv_settle_mode = if shared
            .supported_rcv_settle_modes
            .supports(&remote_attach.rcv_settle_mode)
        {
            remote_attach.rcv_settle_mode.clone()
        } else {
            shared.fallback_rcv_settle_mode.clone()
        };

        // Create channels for Session-Link communication
        let (incoming_tx, mut incoming_rx) = mpsc::channel::<LinkIncomingItem>(shared.buffer_size);

        // Create shared flow state
        let flow_state_inner = LinkFlowStateInner {
            initial_delivery_count: 0, // This will be set in `on_incoming_attach`
            delivery_count: 0,
            link_credit: 0, // The link-credit and available variables are initialized to zero.
            available: 0,
            drain: false, // The drain flag is initialized to false.
            properties: shared.properties.clone(), // Will be set in `on_incoming_attach`
        };
        let flow_state = Arc::new(LinkFlowState::receiver(flow_state_inner));
        let flow_state_producer = flow_state.clone();
        let flow_state_consumer = flow_state;

        // Comparing unsettled should be taken care of in `on_incoming_attach`
        let unsettled = Arc::new(RwLock::new(None));
        let link_handle = LinkRelay::Receiver {
            tx: incoming_tx,
            output_handle: (),
            flow_state: flow_state_producer,
            unsettled: unsettled.clone(),
            receiver_settle_mode: rcv_settle_mode.clone(),
            more: false,
        };

        // Allocate link in session
        let input_handle = InputHandle::from(remote_attach.handle.clone());
        let output_handle = super::session::allocate_incoming_link(
            &control,
            remote_attach.name.clone(),
            link_handle,
            input_handle,
        )
        .await?;

        let mut err = None;
        // **the receiver is considered to hold the authoritative version of the target properties**,
        let local_target = remote_attach
            .target
            .clone()
            .map(|t| T::try_from(*t))
            .transpose()
            .map(|target| {
                target.and_then(|mut t| {
                    if matches!(t.is_dynamic(), Some(true)) {
                        (self.on_dynamic_target)(t).map(|mut t| {
                            *t.capabilities_mut() =
                                self.target_capabilities.clone().map(Into::into);
                            t
                        })
                    } else {
                        *t.capabilities_mut() = self.target_capabilities.clone().map(Into::into);
                        Some(t)
                    }
                })
            })
            .unwrap_or_else(|_| {
                err = Some(ReceiverAttachError::CoordinatorIsNotImplemented);
                None
            });

        let mut link = ReceiverLink::<T> {
            role: PhantomData,
            local_state: LinkState::Unattached, // State change will be taken care of in `on_incoming_attach`
            name: remote_attach.name.clone(),
            output_handle: Some(output_handle),
            input_handle: None, // will be set in `on_incoming_attach`
            snd_settle_mode,
            rcv_settle_mode,
            source: None,         // Will take value from incoming attach
            target: local_target, // Will take value from incoming attach
            max_message_size: shared.max_message_size.unwrap_or(0),
            offered_capabilities: shared.offered_capabilities.clone(),
            desired_capabilities: shared.desired_capabilities.clone(),
            flow_state: flow_state_consumer,
            unsettled,
            verify_incoming_source: self.verify_incoming_source,
            verify_incoming_target: self.verify_incoming_target,
        };

        // The reply must mirror whether the peer is resuming; capture this
        // before `on_incoming_attach` consumes the attach.
        let peer_is_resuming = remote_attach.unsettled.is_some();
        let reply_mode = AttachMode::Reply { peer_is_resuming };

        // `on_incoming_attach` should always be evaluated
        match (err, link.on_incoming_attach(remote_attach)) {
            (Some(attach_error), _) | (_, Err(attach_error)) => {
                // Complete attach anyway
                link.send_attach(&outgoing, &control, reply_mode).await?;
                match attach_error {
                    // ReceiverAttachError::SndSettleModeNotSupported
                    ReceiverAttachError::RcvSettleModeNotSupported => {
                        // FIXME: Ths initiating end should be responsible for checking whether the mode is supported
                    }
                    _ => {
                        return Err(link
                            .handle_attach_error(
                                attach_error,
                                &outgoing,
                                &mut incoming_rx,
                                &control,
                            )
                            .await)
                    }
                }
            }
            _ => link.send_attach(&outgoing, &control, reply_mode).await?,
        }

        let mut inner = ReceiverInner {
            link,
            buffer_size: shared.buffer_size,
            credit_mode: self.credit_mode.clone(),
            processed: AtomicU32::new(0),
            auto_accept: self.auto_accept,
            session: control.clone(),
            outgoing,
            incoming: incoming_rx,
            incomplete_transfer: None,
        };

        if let CreditMode::Auto(credit) = inner.credit_mode {
            #[cfg(feature = "tracing")]
            tracing::debug!("Setting credits");
            #[cfg(feature = "log")]
            log::debug!("Setting credits");
            inner.set_credit(credit).await?;
        }

        Ok(inner)
    }
}

#[cfg(test)]
mod tests {
    use fe2o3_amqp_types::{
        definitions::{DeliveryTag, Handle, ReceiverSettleMode, Role, SenderSettleMode},
        messaging::{DeliveryState, Source, Target, TargetArchetype},
        performatives::Attach,
        primitives::{OrderedMap, Symbol},
    };
    use tokio::sync::{mpsc, oneshot};

    use super::*;
    use crate::{
        endpoint::OutputHandle,
        session::{error::Error, SessionHandle},
    };

    fn remote_sender_attach(
        unsettled: Option<OrderedMap<DeliveryTag, Option<DeliveryState>>>,
        incomplete_unsettled: bool,
    ) -> Attach {
        Attach {
            name: String::from("test-receiver"),
            handle: Handle(0),
            role: Role::Sender,
            snd_settle_mode: SenderSettleMode::Mixed,
            rcv_settle_mode: ReceiverSettleMode::First,
            source: Some(Box::new(Source::default())),
            target: Some(Box::new(TargetArchetype::Target(Target::default()))),
            unsettled,
            incomplete_unsettled,
            initial_delivery_count: Some(0),
            max_message_size: None,
            offered_capabilities: None,
            desired_capabilities: None,
            properties: None,
        }
    }

    /// Drive the acceptor through its real `accept_incoming_attach` path with a
    /// fake session and return the reply `Attach` it wrote.
    async fn accept_and_capture_reply(remote_attach: Attach) -> Attach {
        let (control_tx, mut control_rx) = mpsc::channel::<SessionControl>(16);
        let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<LinkFrame>(16);
        let (outcome_tx, outcome) = oneshot::channel::<Result<(), Error>>();
        drop(outcome_tx);

        let mut session = SessionHandle {
            is_ended: false,
            control: control_tx,
            engine_handle: tokio::spawn(async {}),
            outcome,
            outgoing: outgoing_tx,
            link_listener: (),
        };

        let (reply_tx, reply_rx) = oneshot::channel();
        let driver = tokio::spawn(async move {
            loop {
                tokio::select! {
                    control = control_rx.recv() => match control {
                        Some(SessionControl::AllocateIncomingLink { responder, .. }) => {
                            let _ = responder.send(Ok(OutputHandle(1)));
                        }
                        Some(_) => {}
                        None => break,
                    },
                    frame = outgoing_rx.recv() => {
                        if let Some(frame) = frame {
                            let _ = reply_tx.send(frame);
                        }
                        break;
                    }
                }
            }
        });

        let acceptor = LocalReceiverLinkAcceptor::<Symbol, Target, _>::default();
        let shared = SharedLinkAcceptorFields::default();
        let receiver = acceptor
            .accept_incoming_attach(&shared, remote_attach, &mut session)
            .await
            .expect("accepting the incoming receiver link failed");
        let frame = reply_rx.await.expect("the acceptor did not send a reply");
        drop(receiver);
        driver.abort();

        match frame {
            LinkFrame::Attach(attach) => attach,
            other => panic!("expected an Attach reply, got {other:?}"),
        }
    }

    /// Even when `on_incoming_attach` fails, the receiver acceptor still sends
    /// the reply attach with the peer's resumption flag before driving the
    /// detach handshake.
    #[tokio::test]
    async fn error_path_reply_still_mirrors_peer_resume_map() {
        let (control_tx, mut control_rx) = mpsc::channel::<SessionControl>(16);
        let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<LinkFrame>(16);
        let (outcome_tx, outcome) = oneshot::channel::<Result<(), Error>>();
        drop(outcome_tx);
        let mut session = SessionHandle {
            is_ended: false,
            control: control_tx,
            engine_handle: tokio::spawn(async {}),
            outcome,
            outgoing: outgoing_tx,
            link_listener: (),
        };

        let (reply_tx, reply_rx) = oneshot::channel();
        let driver = tokio::spawn(async move {
            let mut link_tx = None;
            let mut reply_tx = Some(reply_tx);
            loop {
                tokio::select! {
                    control = control_rx.recv() => match control {
                        Some(SessionControl::AllocateIncomingLink { link_relay, responder, .. }) => {
                            if let LinkRelay::Receiver { tx, .. } = link_relay {
                                link_tx = Some(tx);
                            }
                            let _ = responder.send(Ok(OutputHandle(1)));
                        }
                        Some(_) => {}
                        None => break,
                    },
                    frame = outgoing_rx.recv() => match frame {
                        Some(frame @ LinkFrame::Attach(_)) => {
                            if let Some(reply_tx) = reply_tx.take() {
                                let _ = reply_tx.send(frame);
                            }
                        }
                        Some(LinkFrame::Detach(_)) => {
                            if let Some(tx) = &link_tx {
                                let _ = tx
                                    .send(LinkFrame::Detach(
                                        fe2o3_amqp_types::performatives::Detach {
                                            handle: Handle(0),
                                            closed: true,
                                            error: None,
                                        },
                                    ))
                                    .await;
                            }
                            break;
                        }
                        Some(_) => {}
                        None => break,
                    },
                }
            }
        });

        // A missing source makes `on_incoming_attach` fail after the peer's
        // resumption flag has been captured.
        let mut remote_attach = remote_sender_attach(Some(OrderedMap::new()), false);
        remote_attach.source = None;
        let acceptor = LocalReceiverLinkAcceptor::<Symbol, Target, _>::default();
        let shared = SharedLinkAcceptorFields::default();
        let result = acceptor
            .accept_incoming_attach(&shared, remote_attach, &mut session)
            .await;
        let reply = reply_rx.await.expect("the error path did not send a reply");
        driver.abort();

        assert!(matches!(
            result,
            Err(ReceiverAttachError::IncomingSourceIsNone)
        ));
        let reply = match reply {
            LinkFrame::Attach(attach) => attach,
            other => panic!("expected an Attach reply, got {other:?}"),
        };
        assert!(
            reply.unsettled.as_ref().is_some_and(|map| map.is_empty()),
            "the error-path reply must still mirror the peer's resumption"
        );
    }

    /// The receiver acceptor must mirror the peer's resumption state exactly
    /// like the sender acceptor does.
    #[tokio::test]
    async fn reply_mirrors_peer_resume_map() {
        let reply = accept_and_capture_reply(remote_sender_attach(Some(OrderedMap::new()), false))
            .await;
        assert!(reply.unsettled.as_ref().is_some_and(|map| map.is_empty()));
        assert!(!reply.incomplete_unsettled);

        let reply = accept_and_capture_reply(remote_sender_attach(Some(OrderedMap::new()), true))
            .await;
        assert!(reply.unsettled.as_ref().is_some_and(|map| map.is_empty()));

        let reply = accept_and_capture_reply(remote_sender_attach(None, false)).await;
        assert!(reply.unsettled.is_none());
    }
}
