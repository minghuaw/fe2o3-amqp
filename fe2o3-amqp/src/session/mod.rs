//! Implements AMQP1.0 Session

use std::collections::HashMap;

use async_trait::async_trait;
use fe2o3_amqp_types::{
    definitions::{
        self, DeliveryNumber, DeliveryTag, Fields, Handle, Role, SequenceNo, TransferNumber,
    },
    performatives::{Attach, Begin, Detach, Disposition, End, Flow, Transfer},
    primitives::{Symbol, UInt},
    states::SessionState,
};
use slab::Slab;
use tokio::{
    sync::{
        mpsc::{self},
        oneshot,
    },
    task::JoinHandle,
};
use tracing::{instrument, trace};

use crate::{
    connection::ConnectionHandle,
    control::SessionControl,
    endpoint::{self, IncomingChannel, InputHandle, LinkFlow, OutgoingChannel, OutputHandle},
    link::{LinkFrame, LinkRelay},
    util::{is_consecutive, Constant},
    Payload,
};

#[cfg(feature = "transaction")]
use fe2o3_amqp_types::{messaging::Accepted, transaction::TransactionError};

#[cfg(feature = "transaction")]
use crate::{
    endpoint::{HandleDeclare, HandleDischarge},
    transaction::AllocTxnIdError,
};

pub(crate) mod engine;
pub(crate) mod frame;

mod error;
pub(crate) use error::{AllocLinkError, SessionInnerError, SessionStateError};
pub use error::{BeginError, Error};

mod builder;
pub use builder::*;

use self::frame::{SessionFrame, SessionFrameBody};

/// Default incoming_window and outgoing_window
pub const DEFAULT_WINDOW: UInt = 2048;

/// A handle to the [`Session`] event loop
///
/// Dropping the handle will also stop the [`Session`] event loop
#[allow(dead_code)]
pub struct SessionHandle<R> {
    pub(crate) control: mpsc::Sender<SessionControl>,
    pub(crate) engine_handle: JoinHandle<Result<(), Error>>,

    // outgoing for Link
    pub(crate) outgoing: mpsc::Sender<LinkFrame>,
    pub(crate) link_listener: R,
}

impl<R> std::fmt::Debug for SessionHandle<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionHandle").finish()
    }
}

impl<R> Drop for SessionHandle<R> {
    fn drop(&mut self) {
        let _ = self.control.try_send(SessionControl::End(None));
    }
}

impl<R> SessionHandle<R> {
    /// Checks if the underlying event loop has stopped
    pub fn is_ended(&self) -> bool {
        self.control.is_closed()
    }

    /// End the session
    ///
    /// # Panics
    ///
    /// Panics if called after any of [`end`](#method.end), [`end_with_error`](#method.end_with_error),
    /// [`on_end`](#on_end) has beend executed.
    /// This will cause the JoinHandle to be polled after completion, which causes a panic.
    pub async fn end(&mut self) -> Result<(), Error> {
        // If sending is unsuccessful, the `SessionEngine` event loop is
        // already dropped, this should be reflected by `JoinError` then.
        let _ = self.control.send(SessionControl::End(None)).await;
        self.on_end().await
    }

    /// Alias for [`end`](#method.end)
    pub async fn close(&mut self) -> Result<(), Error> {
        self.end().await
    }

    /// End the session with an error
    ///
    /// # Panics
    ///
    /// Panics if called after any of [`end`](#method.end), [`end_with_error`](#method.end_with_error),
    /// [`on_end`](#on_end) has beend executed.    
    /// This will cause the JoinHandle to be polled after completion, which causes a panic.
    pub async fn end_with_error(
        &mut self,
        error: impl Into<definitions::Error>,
    ) -> Result<(), Error> {
        // If sending is unsuccessful, the `SessionEngine` event loop is
        // already dropped, this should be reflected by `JoinError` then.
        let _ = self
            .control
            .send(SessionControl::End(Some(error.into())))
            .await;
        self.on_end().await
    }

    /// Returns when the underlying event loop has stopped
    ///
    /// # Panics
    ///
    /// Panics if called after any of [`end`](#method.end), [`end_with_error`](#method.end_with_error),
    /// [`on_end`](#on_end) has beend executed.
    /// This will cause the JoinHandle to be polled after completion, which causes a panic.
    pub async fn on_end(&mut self) -> Result<(), Error> {
        match (&mut self.engine_handle).await {
            Ok(res) => {
                res?;
                Ok(())
            }
            Err(join_error) => Err(Error::JoinError(join_error)),
        }
    }
}

/// # Cancel safety
///
/// It internally `.await` on a send on `tokio::mpsc::Sender` and on a `oneshot::Receiver`.
/// This should be cancel safe
pub(crate) async fn allocate_link(
    control: &mpsc::Sender<SessionControl>,
    link_name: String,
    link_relay: LinkRelay<()>,
) -> Result<OutputHandle, AllocLinkError> {
    let (responder, resp_rx) = oneshot::channel();

    control
        .send(SessionControl::AllocateLink {
            link_name,
            link_relay,
            responder,
        })
        .await // cancel safe
        // The `SendError` could only happen when the receiving half is
        // dropped, meaning the `SessionEngine::event_loop` has stopped.
        // This would also mean the `Session` is Unmapped, and thus it
        // may be treated as illegal state
        .map_err(|_| AllocLinkError::IllegalSessionState)?;
    resp_rx
        .await // FIXME: Is oneshot channel cancel safe?
        // The error could only occur when the sending half is dropped,
        // indicating the `SessionEngine::even_loop` has stopped or
        // unmapped. Thus it could be considered as illegal state
        .map_err(|_| AllocLinkError::IllegalSessionState)?
}

/// AMQP1.0 Session
///
/// # Begin a new Session with default configuration
///
/// ```rust,ignore
/// use fe2o3_amqp::Session;
///
/// let session = Session::begin(&mut connection).await.unwrap();
/// ```
///
/// ## Default configuration
///
/// | Field | Default Value |
/// |-------|---------------|
/// |`next_outgoing_id`| 0 |
/// |`incoming_window`| [`DEFAULT_WINDOW`] |
/// |`outgoing_window`| [`DEFAULT_WINDOW`] |
/// |`handle_max`| `u32::MAX` |
/// |`offered_capabilities` | `None` |
/// |`desired_capabilities`| `None` |
/// |`Properties`| `None` |
///
/// # Customize configuration with [`Builder`]
///
/// The builder should be used if the user would like to customize the configuration
/// for the session.
///
/// ```rust, ignore
/// let session = Session::builder()
///     .handle_max(128)
///     .begin(&mut connection)
///     .await.unwrap();
/// ```
///
#[derive(Debug)]
pub struct Session {
    pub(crate) outgoing_channel: OutgoingChannel,

    // local amqp states
    pub(crate) local_state: SessionState,
    pub(crate) initial_outgoing_id: Constant<TransferNumber>,
    pub(crate) next_outgoing_id: TransferNumber,
    pub(crate) incoming_window: TransferNumber,
    pub(crate) outgoing_window: TransferNumber,
    pub(crate) handle_max: Handle,

    // remote amqp states
    pub(crate) incoming_channel: Option<IncomingChannel>,
    // initialize with 0 first and change after receiving the remote Begin
    pub(crate) next_incoming_id: TransferNumber,
    pub(crate) remote_incoming_window: SequenceNo,
    pub(crate) remote_outgoing_window: SequenceNo,

    // capabilities
    pub(crate) offered_capabilities: Option<Vec<Symbol>>,
    pub(crate) desired_capabilities: Option<Vec<Symbol>>,
    pub(crate) properties: Option<Fields>,

    // local links by output handle
    pub(crate) link_name_by_output_handle: Slab<String>,
    pub(crate) link_by_name: HashMap<String, Option<LinkRelay<OutputHandle>>>,
    pub(crate) link_by_input_handle: HashMap<InputHandle, LinkRelay<OutputHandle>>,
    // Maps from DeliveryId to link.DeliveryCount
    pub(crate) delivery_tag_by_id: HashMap<(Role, DeliveryNumber), (InputHandle, DeliveryTag)>, // Role must be the remote peer's role
}

impl Session {
    /// Creates a builder for [`Session`]
    pub fn builder() -> builder::Builder {
        builder::Builder::new()
    }

    /// Begins a new session with the default configurations
    ///
    /// # Default configuration
    ///
    /// | Field | Default Value |
    /// |-------|---------------|
    /// |`next_outgoing_id`| 0 |
    /// |`incoming_window`| [`DEFAULT_WINDOW`] |
    /// |`outgoing_window`| [`DEFAULT_WINDOW`] |
    /// |`handle_max`| `u32::MAX` |
    /// |`offered_capabilities` | `None` |
    /// |`desired_capabilities`| `None` |
    /// |`Properties`| `None` |
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use fe2o3_amqp::Session;
    ///
    /// let session = Session::begin(&mut connection).await.unwrap();
    /// ```
    pub async fn begin(conn: &mut ConnectionHandle<()>) -> Result<SessionHandle<()>, BeginError> {
        Session::builder().begin(conn).await
    }
}

impl endpoint::SessionExt for Session {}

#[async_trait]
impl endpoint::Session for Session {
    type AllocError = AllocLinkError;
    type BeginError = SessionStateError;
    type EndError = SessionStateError;
    type Error = SessionInnerError;
    type State = SessionState;

    fn local_state(&self) -> &Self::State {
        &self.local_state
    }

    fn local_state_mut(&mut self) -> &mut Self::State {
        &mut self.local_state
    }

    fn outgoing_channel(&self) -> OutgoingChannel {
        self.outgoing_channel
    }

    fn allocate_link(
        &mut self,
        link_name: String,
        link_relay: Option<LinkRelay<()>>,
    ) -> Result<OutputHandle, Self::AllocError> {
        match &self.local_state {
            SessionState::Mapped => {}
            _ => return Err(AllocLinkError::IllegalSessionState),
        };

        // check whether link name is duplciated
        if self.link_by_name.contains_key(&link_name) {
            return Err(AllocLinkError::DuplicatedLinkName);
        }

        // get a new entry index
        let entry = self.link_name_by_output_handle.vacant_entry();
        let handle = OutputHandle(entry.key() as u32);

        entry.insert(link_name.clone());
        let value = link_relay.map(|val| val.with_output_handle(handle.clone()));
        self.link_by_name.insert(link_name, value);
        Ok(handle)
    }

    fn allocate_incoming_link(
        &mut self,
        link_name: String,
        link_relay: LinkRelay<()>,
        input_handle: InputHandle,
    ) -> Result<OutputHandle, Self::AllocError> {
        match self.allocate_link(link_name, None) {
            Ok(output_handle) => {
                let value = link_relay.with_output_handle(output_handle.clone());
                self.link_by_input_handle.insert(input_handle, value);
                Ok(output_handle)
            }
            Err(err) => Err(err),
        }
    }

    /// This should only deallocate the output handle
    fn deallocate_link(&mut self, output_handle: OutputHandle) {
        if let Some(name) = self
            .link_name_by_output_handle
            .try_remove(output_handle.0 as usize)
        {
            let _ = self.link_by_name.remove(&name);
        }
    }

    fn on_incoming_begin(
        &mut self,
        channel: IncomingChannel,
        begin: Begin,
    ) -> Result<(), Self::BeginError> {
        match self.local_state {
            SessionState::Unmapped => self.local_state = SessionState::BeginReceived,
            SessionState::BeginSent => self.local_state = SessionState::Mapped,
            _ => return Err(SessionStateError::IllegalState), // End session with unattached handle?
        }

        self.incoming_channel = Some(channel);
        self.next_incoming_id = begin.next_outgoing_id;
        self.remote_incoming_window = begin.incoming_window;
        self.remote_outgoing_window = begin.outgoing_window;

        Ok(())
    }

    async fn on_incoming_attach(&mut self, attach: Attach) -> Result<(), Self::Error> {
        match self.link_by_name.get_mut(&attach.name) {
            Some(link) => match link.take() {
                Some(mut relay) => {
                    // Only Sender need to update the receiver settle mode
                    // because the sender needs to echo a disposition if
                    // rcv-settle-mode is 1
                    if let LinkRelay::Sender {
                        receiver_settle_mode,
                        ..
                    } = &mut relay
                    {
                        *receiver_settle_mode = attach.rcv_settle_mode.clone();
                    }

                    let input_handle = InputHandle::from(attach.handle.clone()); // handle is just a wrapper around u32
                    relay
                        .send(LinkFrame::Attach(attach))
                        .await
                        .map_err(|_| SessionInnerError::UnattachedHandle)?;
                    self.link_by_input_handle.insert(input_handle, relay);

                    Ok(())
                }
                None => {
                    // Link name is found but is already in use
                    Err(SessionInnerError::HandleInUse)
                }
            },
            None => Err(SessionInnerError::RemoteAttachingLinkNameNotFound), // End session with unattached handle?,
        }
    }

    async fn on_incoming_flow(&mut self, flow: Flow) -> Result<Option<LinkFlow>, Self::Error> {
        // Handle session flow control
        //
        // When the endpoint receives a flow frame from its peer, it MUST update the next-incoming-id
        // directly from the next-outgoing-id of the frame, and it MUST update the remote-outgoing-
        // window directly from the outgoing-window of the frame.
        self.next_incoming_id = flow.next_outgoing_id;
        self.remote_outgoing_window = flow.outgoing_window;

        match &flow.next_incoming_id {
            Some(flow_next_incoming_id) => {
                // The remote-incoming-window is computed as follows:
                // next-incoming-id_flow + incoming-window_flow - next-outgoing-id_endpoint
                self.remote_incoming_window =
                    flow_next_incoming_id + flow.incoming_window - self.next_outgoing_id;
            }
            None => {
                // If the next-incoming-id field of the flow frame is not set, then remote-incoming-window is computed as follows:
                // initial-outgoing-id_endpoint + incoming-window_flow - next-outgoing-id_endpoint
                self.remote_incoming_window = *(self.initial_outgoing_id.value())
                    + flow.incoming_window
                    - self.next_outgoing_id;
            }
        }

        // Handle link flow control
        if let Ok(link_flow) = LinkFlow::try_from(flow) {
            let input_handle = InputHandle::from(link_flow.handle.clone());
            match self.link_by_input_handle.get_mut(&input_handle) {
                Some(link_relay) => {
                    return link_relay
                        .on_incoming_flow(link_flow)
                        .await
                        .map_err(Into::into);
                }
                None => return Err(SessionInnerError::UnattachedHandle), // End session with unattached handle?
            }
        }

        Ok(None)
    }

    async fn on_incoming_transfer(
        &mut self,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<Option<Disposition>, Self::Error> {
        // Upon receiving a transfer, the receiving endpoint will increment the next-incoming-id to
        // match the implicit transfer-id of the incoming transfer plus one, as well as decrementing the
        // remote-outgoing-window, and MAY (depending on policy) decrement its incoming-window.

        self.next_incoming_id += 1;
        self.remote_outgoing_window -= 1;

        let input_handle = InputHandle::from(transfer.handle.clone());
        match self.link_by_input_handle.get_mut(&input_handle) {
            Some(link_relay) => {
                let id_and_tag = link_relay.on_incoming_transfer(transfer, payload).await?;

                // FIXME: If the unsettled map needs this
                if let Some((delivery_id, delivery_tag)) = id_and_tag {
                    self.delivery_tag_by_id
                        .insert((Role::Sender, delivery_id), (input_handle, delivery_tag));
                }
            }
            None => return Err(SessionInnerError::UnattachedHandle),
        };

        Ok(None)
    }

    #[instrument(skip_all)]
    fn on_incoming_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<Option<Vec<Disposition>>, Self::Error> {
        // and disposition only has delivery id?

        let first = disposition.first;
        let last = disposition.last.unwrap_or(first);

        // A disposition frame may refer to deliveries on multiple links, each may be running
        // in different mode. This counts the largest sections that can be echoed back together
        if disposition.settled {
            // If it is alrea
            for delivery_id in first..=last {
                let key = (disposition.role.clone(), delivery_id);
                if let Some((handle, delivery_tag)) = self.delivery_tag_by_id.remove(&key) {
                    if let Some(link_handle) = self.link_by_input_handle.get_mut(&handle) {
                        let _echo = link_handle.on_incoming_disposition(
                            disposition.role.clone(),
                            disposition.settled,
                            disposition.state.clone(),
                            delivery_tag,
                        );
                    }
                }
            }

            Ok(None)
        } else {
            let mut delivery_ids = Vec::new();
            for delivery_id in first..=last {
                let key = (disposition.role.clone(), delivery_id);
                if let Some((handle, delivery_tag)) = self.delivery_tag_by_id.get(&key) {
                    if let Some(link_handle) = self.link_by_input_handle.get_mut(handle) {
                        // In mode Second, the receiver will first send a non-settled disposition,
                        // and wait for sender's settled disposition
                        let echo = link_handle.on_incoming_disposition(
                            disposition.role.clone(),
                            disposition.settled,
                            disposition.state.clone(),
                            delivery_tag.clone(),
                        );

                        if echo {
                            delivery_ids.push(delivery_id);
                        }
                    }
                }
            }

            let chunk_inds = consecutive_chunk_indices(&delivery_ids[..]);

            let mut dispositions = Vec::with_capacity(chunk_inds.len());
            let mut prev_ind = 0;
            for ind in chunk_inds {
                let slice = &delivery_ids[prev_ind..ind];
                let disposition = Disposition {
                    role: Role::Sender,
                    first: slice[0],
                    last: slice.last().copied(),
                    settled: true,
                    state: disposition.state.clone(),
                    batchable: false,
                };
                dispositions.push(disposition);
                prev_ind = ind;
            }
            Ok(Some(dispositions))
        }
    }

    #[instrument(skip_all)]
    async fn on_incoming_detach(&mut self, detach: Detach) -> Result<(), Self::Error> {
        trace!(frame = ?detach);
        // Remove the link by input handle
        match self
            .link_by_input_handle
            .remove(&InputHandle::from(detach.handle.clone()))
        {
            Some(mut link) => link
                .on_incoming_detach(detach)
                .await
                .map_err(|_| SessionInnerError::UnattachedHandle),
            None => Err(SessionInnerError::UnattachedHandle),
        }
    }

    #[instrument(skip_all)]
    fn on_incoming_end(
        &mut self,
        _channel: IncomingChannel,
        end: End,
    ) -> Result<(), Self::EndError> {
        trace!(end = ?end);
        match self.local_state {
            SessionState::BeginSent | SessionState::BeginReceived | SessionState::Mapped => {
                self.local_state = SessionState::EndReceived;

                match end.error {
                    Some(err) => Err(SessionStateError::RemoteEndedWithError(err)),
                    None => Err(SessionStateError::RemoteEnded),
                }
            }
            SessionState::EndSent | SessionState::Discarding => {
                self.local_state = SessionState::Unmapped;

                if let Some(error) = end.error {
                    tracing::error!(remote_error = ?error);
                    return Err(SessionStateError::RemoteEndedWithError(error));
                }
                Ok(())
            }
            _ => Err(SessionStateError::IllegalState), // End session with illegal state?
        }
    }

    async fn send_begin(
        &mut self,
        writer: &mpsc::Sender<SessionFrame>,
    ) -> Result<(), Self::BeginError> {
        let begin = Begin {
            remote_channel: self.incoming_channel.map(Into::into),
            next_outgoing_id: self.next_outgoing_id,
            incoming_window: self.incoming_window,
            outgoing_window: self.outgoing_window,
            handle_max: self.handle_max.clone(),
            offered_capabilities: self.offered_capabilities.clone().map(Into::into),
            desired_capabilities: self.desired_capabilities.clone().map(Into::into),
            properties: self.properties.clone(),
        };
        let frame = SessionFrame::new(self.outgoing_channel, SessionFrameBody::Begin(begin));

        // check local states
        match &self.local_state {
            SessionState::Unmapped => {
                writer
                    .send(frame)
                    .await
                    // The receiving half must have dropped, and thus the `Connection`
                    // event loop has stopped. It should be treated as an io error
                    .map_err(|_| SessionStateError::IllegalConnectionState)?;
                self.local_state = SessionState::BeginSent;
            }
            SessionState::BeginReceived => {
                writer
                    .send(frame)
                    .await
                    .map_err(|_| SessionStateError::IllegalConnectionState)?;
                self.local_state = SessionState::Mapped;
            }
            _ => return Err(SessionStateError::IllegalState),
        }

        Ok(())
    }

    async fn send_end(
        &mut self,
        writer: &mpsc::Sender<SessionFrame>,
        error: Option<definitions::Error>,
    ) -> Result<(), Self::EndError> {
        match self.local_state {
            SessionState::Mapped => match error.is_some() {
                true => self.local_state = SessionState::Discarding,
                false => self.local_state = SessionState::EndSent,
            },
            SessionState::EndReceived => self.local_state = SessionState::Unmapped,
            _ => return Err(SessionStateError::IllegalState),
        }

        let frame = SessionFrame::new(self.outgoing_channel, SessionFrameBody::End(End { error }));
        writer
            .send(frame)
            .await
            // The receiving half must have dropped, and thus the `Connection`
            // event loop has stopped. It should be treated as an io error
            .map_err(|_| SessionStateError::IllegalConnectionState)?;
        Ok(())
    }

    fn on_outgoing_attach(&mut self, attach: Attach) -> Result<SessionFrame, Self::Error> {
        let body = SessionFrameBody::Attach(attach);
        let frame = SessionFrame::new(self.outgoing_channel, body);
        Ok(frame)
    }

    fn on_outgoing_flow(&mut self, flow: LinkFlow) -> Result<SessionFrame, Self::Error> {
        let flow = Flow {
            // Session flow states
            next_incoming_id: Some(self.next_incoming_id),
            incoming_window: self.incoming_window,
            next_outgoing_id: self.next_outgoing_id,
            outgoing_window: self.outgoing_window,
            // Link flow states
            handle: Some(flow.handle),
            delivery_count: flow.delivery_count,
            link_credit: flow.link_credit,
            available: flow.available,
            drain: flow.drain,
            echo: flow.echo,
            properties: flow.properties,
        };

        let body = SessionFrameBody::Flow(flow);
        let frame = SessionFrame::new(self.outgoing_channel, body);
        Ok(frame)
    }

    fn on_outgoing_transfer(
        &mut self,
        input_handle: InputHandle,
        mut transfer: Transfer,
        payload: Payload,
    ) -> Result<SessionFrame, Self::Error> {
        // Upon sending a transfer, the sending endpoint will increment its next-outgoing-id, decre-
        // ment its remote-incoming-window, and MAY (depending on policy) decrement its outgoing-
        // window.

        // TODO: What policy would result in a decrement in outgoing-window?

        // If not set on the first (or only) transfer for a (multi-transfer)
        // delivery, then the settled flag MUST be interpreted as being false.
        //
        // If the negotiated value for snd-settle-mode at attachment is settled,
        // then this field MUST be true on at least one transfer frame for a
        // delivery
        //
        // If the negotiated value for snd-settle-mode at attachment is unsettled,
        // then this field MUST be false (or unset) on every transfer frame for a
        // delivery
        let settled = transfer.settled.unwrap_or(false);

        // Only the first transfer is required to have delivery_tag and delivery_id
        if let Some(delivery_tag) = &transfer.delivery_tag {
            // The next-outgoing-id is the transfer-id to assign to the next transfer frame.
            let delivery_id = self.next_outgoing_id;
            transfer.delivery_id = Some(delivery_id);

            // Disposition doesn't carry delivery tag
            if !settled {
                self.delivery_tag_by_id.insert(
                    (Role::Receiver, delivery_id),
                    (input_handle, delivery_tag.clone()),
                );
            }
        }

        self.next_outgoing_id += 1;

        // The remote-incoming-window reflects the maximum number of outgoing transfers that can
        // be sent without exceeding the remote endpoint’s incoming-window. This value MUST be
        // decremented after every transfer frame is sent, and recomputed when informed of the
        // remote session endpoint state.
        self.remote_incoming_window -= 1;

        let body = SessionFrameBody::Transfer {
            performative: transfer,
            payload,
        };
        let frame = SessionFrame::new(self.outgoing_channel, body);
        Ok(frame)
    }

    fn on_outgoing_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<SessionFrame, Self::Error> {
        // Currently the sender cannot actively dispose any message
        // because the sender doesn't have access to the delivery_id

        let body = SessionFrameBody::Disposition(disposition);
        let frame = SessionFrame::new(self.outgoing_channel, body);
        Ok(frame)
    }

    fn on_outgoing_detach(&mut self, detach: Detach) -> SessionFrame {
        self.deallocate_link(detach.handle.clone().into());
        let body = SessionFrameBody::Detach(detach);
        SessionFrame::new(self.outgoing_channel, body)
    }
}

#[cfg(feature = "transaction")]
impl HandleDeclare for Session {
    // This should be unreachable, but an error is probably a better way
    fn allocate_transaction_id(
        &mut self,
    ) -> Result<fe2o3_amqp_types::transaction::TransactionId, AllocTxnIdError> {
        // Err(Error::amqp_error(AmqpError::NotImplemented, "Resource side transaction is not enabled".to_string()))
        Err(AllocTxnIdError::NotImplemented)
    }
}

#[cfg(feature = "transaction")]
#[async_trait]
impl HandleDischarge for Session {
    async fn commit_transaction(
        &mut self,
        _txn_id: fe2o3_amqp_types::transaction::TransactionId,
    ) -> Result<Result<Accepted, TransactionError>, Self::Error> {
        // FIXME: This should be impossible
        Ok(Err(TransactionError::UnknownId))
    }

    fn rollback_transaction(
        &mut self,
        _txn_id: fe2o3_amqp_types::transaction::TransactionId,
    ) -> Result<Result<Accepted, TransactionError>, Self::Error> {
        // FIXME: This should be impossible
        Ok(Err(TransactionError::UnknownId))
    }
}

fn consecutive_chunk_indices(delivery_ids: &[DeliveryNumber]) -> Vec<usize> {
    delivery_ids
        .windows(2)
        .enumerate()
        .filter_map(|(i, id)| {
            if is_consecutive(&id[0], &id[1]) {
                None
            } else {
                Some(i + 1)
            }
        })
        .collect()
}
