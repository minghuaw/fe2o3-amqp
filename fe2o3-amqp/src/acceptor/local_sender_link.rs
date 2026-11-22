//! Implements acceptor for a remote receiver link

use std::{marker::PhantomData, sync::Arc};

use fe2o3_amqp_types::{
    definitions::SequenceNo,
    messaging::{Source, Target},
    performatives::Attach,
    primitives::Symbol,
};
use parking_lot::RwLock;
use tokio::sync::{mpsc, Notify};

use crate::{
    endpoint::{InputHandle, LinkAttach, LinkExt},
    link::{
        sender::SenderInner,
        state::{LinkFlowState, LinkFlowStateInner, LinkState},
        AttachMode, LinkRelay, SenderAttachError, SenderLink,
    },
    session::SessionHandle,
    util::{Consumer, Producer},
    Sender,
};

use super::link::SharedLinkAcceptorFields;

/// An acceptor for a remote receiver link
///
/// the sender is considered to hold the authoritative version of the
/// source properties, the receiver is considered to hold the authoritative version of the target properties.
#[derive(Debug, Clone)]
pub(crate) struct LocalSenderLinkAcceptor<C, F>
where
    F: Fn(Source) -> Option<Source>,
{
    /// This MUST NOT be null if role is sender,
    /// and it is ignored if the role is receiver.
    /// See subsection 2.6.7.
    pub initial_delivery_count: SequenceNo,

    /// the extension capabilities the sender supports/desires
    pub source_capabilities: Option<Vec<C>>,

    pub on_dynamic_source: F,

    /// Whether the local link will verify the incoming source/target
    pub verify_incoming_source: bool,
    pub verify_incoming_target: bool,
}

fn reject_dynamic_source(_: Source) -> Option<Source> {
    None
}

impl<C> Default for LocalSenderLinkAcceptor<C, fn(Source) -> Option<Source>>
where
    C: From<Symbol>,
{
    fn default() -> Self {
        Self {
            initial_delivery_count: 0,
            source_capabilities: None,
            on_dynamic_source: reject_dynamic_source,
            verify_incoming_source: true,
            verify_incoming_target: true,
        }
    }
}

impl<F> LocalSenderLinkAcceptor<Symbol, F>
where
    F: Fn(Source) -> Option<Source>,
{
    /// Accepts an incoming attach as a local sender
    pub async fn accept_incoming_attach<R>(
        &self,
        shared: &SharedLinkAcceptorFields,
        remote_attach: Attach,
        session: &mut SessionHandle<R>,
    ) -> Result<Sender, SenderAttachError> {
        let snd_settle_mode = if shared
            .supported_snd_settle_modes
            .supports(&remote_attach.snd_settle_mode)
        {
            remote_attach.snd_settle_mode.clone()
        } else {
            shared.fallback_snd_settle_mode.clone()
        };
        let rcv_settle_mode = if shared
            .supported_rcv_settle_modes
            .supports(&remote_attach.rcv_settle_mode)
        {
            remote_attach.rcv_settle_mode.clone()
        } else {
            shared.fallback_rcv_settle_mode.clone()
        };

        let (incoming_tx, mut incoming_rx) = mpsc::channel(shared.buffer_size);

        let flow_state_inner = LinkFlowStateInner {
            initial_delivery_count: self.initial_delivery_count,
            delivery_count: self.initial_delivery_count,
            link_credit: 0,
            available: 0,
            drain: false,
            properties: shared.properties.clone(),
        };
        let flow_state = Arc::new(LinkFlowState::sender(flow_state_inner));
        let notifier = Arc::new(Notify::new());
        let flow_state_producer = Producer::new(notifier.clone(), flow_state.clone());
        let flow_state_consumer = Consumer::new(notifier, flow_state);

        let unsettled = Arc::new(RwLock::new(None));
        let link_handle = LinkRelay::Sender {
            tx: incoming_tx,
            output_handle: (),
            flow_state: flow_state_producer,
            unsettled: unsettled.clone(),
            receiver_settle_mode: remote_attach.rcv_settle_mode.clone(),
        };

        // Allocate link in session
        let input_handle = InputHandle::from(remote_attach.handle.clone());
        let output_handle = super::session::allocate_incoming_link(
            &session.control,
            remote_attach.name.clone(),
            link_handle,
            input_handle,
        )
        .await?;

        // In this case, the sender is considered to hold the authoritative version of the
        // version of the source properties
        let local_source = remote_attach.source.clone().and_then(|s| {
            if s.dynamic {
                (self.on_dynamic_source)(*s).map(|mut s| {
                    s.capabilities = self.source_capabilities.clone().map(Into::into);
                    s
                })
            } else {
                let mut source = *s;
                source.capabilities = self.source_capabilities.clone().map(Into::into);
                Some(source)
            }
        });

        let mut link = SenderLink::<Target> {
            role: PhantomData,
            local_state: LinkState::Unattached, // will be set in `on_incoming_attach`

            name: remote_attach.name.clone(),
            output_handle: Some(output_handle),
            input_handle: None, // this will be set in `on_incoming_attach`
            snd_settle_mode,
            rcv_settle_mode,
            source: local_source,
            target: None, // Will take value from incoming attach
            max_message_size: shared.max_message_size.unwrap_or(0),
            offered_capabilities: shared.offered_capabilities.clone(),
            desired_capabilities: shared.desired_capabilities.clone(),
            flow_state: flow_state_consumer,
            unsettled,
            verify_incoming_source: self.verify_incoming_source,
            verify_incoming_target: self.verify_incoming_target,
        };

        let outgoing = session.outgoing.clone();
        // The reply must mirror whether the peer is resuming; capture this
        // before `on_incoming_attach` consumes the attach.
        let peer_is_resuming = remote_attach.unsettled.is_some();
        let reply_mode = AttachMode::Reply { peer_is_resuming };

        match link.on_incoming_attach(remote_attach) {
            Ok(_) => link.send_attach(&outgoing, &session.control, reply_mode).await?,
            Err(attach_error) => {
                // Complete attach then detach should any error happen
                link.send_attach(&outgoing, &session.control, reply_mode).await?;
                match attach_error {
                    SenderAttachError::SndSettleModeNotSupported
                    | SenderAttachError::RcvSettleModeNotSupported => {
                        // FIXME: The initiating side is responsible for checking whether the desired modes are supported?
                    }
                    _ => {
                        return Err(link
                            .handle_attach_error(
                                attach_error,
                                &outgoing,
                                &mut incoming_rx,
                                &session.control,
                            )
                            .await);
                    }
                }
            }
        }

        let inner = SenderInner {
            link,
            buffer_size: shared.buffer_size,
            session: session.control.clone(),
            outgoing,
            incoming: incoming_rx,
            pending_redeliveries: Vec::new(),
        };
        Ok(Sender { inner })
    }
}

#[cfg(test)]
mod tests {
    use fe2o3_amqp_types::{
        definitions::{DeliveryTag, Handle, ReceiverSettleMode, Role, SenderSettleMode},
        messaging::{DeliveryState, Target, TargetArchetype},
        performatives::Attach,
        primitives::{OrderedMap, Symbol},
    };
    use tokio::sync::{mpsc, oneshot};

    use super::*;
    use crate::{
        control::SessionControl,
        endpoint::OutputHandle,
        link::LinkFrame,
        session::{error::Error, SessionHandle},
    };

    fn remote_receiver_attach(
        unsettled: Option<OrderedMap<DeliveryTag, Option<DeliveryState>>>,
        incomplete_unsettled: bool,
    ) -> Attach {
        Attach {
            name: String::from("test-sender"),
            handle: Handle(0),
            role: Role::Receiver,
            snd_settle_mode: SenderSettleMode::Mixed,
            rcv_settle_mode: ReceiverSettleMode::First,
            source: None,
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

        let acceptor = LocalSenderLinkAcceptor::<Symbol, _>::default();
        let shared = SharedLinkAcceptorFields::default();
        let sender = acceptor
            .accept_incoming_attach(&shared, remote_attach, &mut session)
            .await
            .expect("accepting the incoming sender link failed");
        let frame = reply_rx.await.expect("the acceptor did not send a reply");
        drop(sender);
        driver.abort();

        match frame {
            LinkFrame::Attach(attach) => attach,
            other => panic!("expected an Attach reply, got {other:?}"),
        }
    }

    /// Even when `on_incoming_attach` fails, the acceptor still sends the
    /// reply attach with the peer's resumption flag before driving the detach
    /// handshake.
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
                            if let LinkRelay::Sender { tx, .. } = link_relay {
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
                                    .send(LinkFrame::Detach(fe2o3_amqp_types::performatives::Detach {
                                        handle: Handle(0),
                                        closed: true,
                                        error: None,
                                    }))
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

        // A missing target makes `on_incoming_attach` fail after the peer's
        // resumption flag has been captured.
        let mut remote_attach = remote_receiver_attach(Some(OrderedMap::new()), false);
        remote_attach.target = None;
        let acceptor = LocalSenderLinkAcceptor::<Symbol, _>::default();
        let shared = SharedLinkAcceptorFields::default();
        let result = acceptor
            .accept_incoming_attach(&shared, remote_attach, &mut session)
            .await;
        let reply = reply_rx.await.expect("the error path did not send a reply");
        driver.abort();

        assert!(matches!(result, Err(SenderAttachError::IncomingTargetIsNone)));
        let reply = match reply {
            LinkFrame::Attach(attach) => attach,
            other => panic!("expected an Attach reply, got {other:?}"),
        };
        assert!(
            reply.unsettled.as_ref().is_some_and(|map| map.is_empty()),
            "the error-path reply must still mirror the peer's resumption"
        );
    }

    /// The acceptor must mirror the peer's resumption state: a peer attach
    /// with a non-null unsettled map gets a non-null reply map (even when the
    /// local map is empty), while a null peer map gets a null reply.
    #[tokio::test]
    async fn reply_mirrors_peer_resume_map() {
        let reply = accept_and_capture_reply(remote_receiver_attach(
            Some(OrderedMap::new()),
            false,
        ))
        .await;
        assert!(
            reply.unsettled.as_ref().is_some_and(|map| map.is_empty()),
            "a resuming peer must receive a non-null (empty) reply map"
        );
        assert!(!reply.incomplete_unsettled);

        // The peer marked its map incomplete: the local state is
        // `IncompleteAttachReceived`, which must still accept the reply.
        let reply = accept_and_capture_reply(remote_receiver_attach(
            Some(OrderedMap::new()),
            true,
        ))
        .await;
        assert!(
            reply.unsettled.as_ref().is_some_and(|map| map.is_empty()),
            "an incomplete resuming peer must still receive a non-null reply map"
        );

        let reply = accept_and_capture_reply(remote_receiver_attach(None, false)).await;
        assert!(
            reply.unsettled.is_none(),
            "a non-resuming peer must receive a null reply map"
        );
    }
}
