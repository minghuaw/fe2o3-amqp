//! Session Listener

use async_trait::async_trait;
use fe2o3_amqp_types::{
    definitions::{self, ConnectionError},
    performatives::{Attach, Begin, Detach, Disposition, End, Flow, Transfer},
    states::SessionState,
};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tracing::instrument;

use crate::{
    connection::AllocSessionError,
    control::{ConnectionControl, SessionControl},
    endpoint::{
        self, IncomingChannel, InputHandle, LinkFlow, OutgoingChannel, OutputHandle, Session,
    },
    link::{LinkFrame, LinkRelay},
    session::{
        self,
        engine::SessionEngine,
        frame::{SessionFrame, SessionIncomingItem},
        AllocLinkError, BeginError, Error, SessionHandle,
        DEFAULT_SESSION_CONTROL_BUFFER_SIZE,
    },
    util::Initialized,
    Payload,
};

use super::{builder::Builder, IncomingSession, ListenerConnectionHandle};

#[cfg(feature = "transaction")]
use fe2o3_amqp_types::{messaging::Accepted, transaction::TransactionError};

#[cfg(feature = "transaction")]
use crate::transaction::{manager::TransactionManager, session::TxnSession, AllocTxnIdError};

/// An empty marker trait that acts as a constraint for session engine
pub trait ListenerSessionEndpoint {}

impl ListenerSessionEndpoint for ListenerSession {}

#[cfg(feature = "transaction")]
impl ListenerSessionEndpoint for TxnSession<ListenerSession> {}

type SessionBuilder = crate::session::Builder;

/// Type alias for listener session handle
pub type ListenerSessionHandle = SessionHandle<mpsc::Receiver<Attach>>;

impl ListenerSessionHandle {
    /// Waits for the next incoming link
    pub async fn next_incoming_attach(&mut self) -> Option<Attach> {
        self.link_listener.recv().await
    }
}

pub(crate) async fn allocate_incoming_link(
    control: &mpsc::Sender<SessionControl>,
    link_name: String,
    link_relay: LinkRelay<()>,
    input_handle: InputHandle,
) -> Result<OutputHandle, AllocLinkError> {
    let (responder, resp_rx) = oneshot::channel();

    control
        .send(SessionControl::AllocateIncomingLink {
            link_name,
            link_relay,
            input_handle,
            responder,
        })
        .await
        // The `SendError` could only happen when the receiving half is
        // dropped, meaning the `SessionEngine::event_loop` has stopped.
        // This would also mean the `Session` is Unmapped, and thus it
        // may be treated as illegal state
        .map_err(|_| AllocLinkError::IllegalSessionState)?;
    let result = resp_rx
        .await
        // The error could only occur when the sending half is dropped,
        // indicating the `SessionEngine::even_loop` has stopped or
        // unmapped. Thus it could be considered as illegal state
        .map_err(|_| AllocLinkError::IllegalSessionState)?;
    result
}

/// An acceptor for incoming session
///
/// This is simply a wrapper around the session builder since there is not
/// much else that you can configure. The wrapper is here for consistency in terms of API desgin.
///
/// # Accepts incoming session with default configuration
///
/// ```rust,ignore
/// use crate::acceptor::SessionAcceptor;
///
/// let mut connection: ListenerConnectionHandle = connection_acceptor.accept(stream).await.unwrap();
/// let session_acceptor = SessionAcceptor::new();
/// let session = session_acceptor.accept(&mut connection).await.unwrap();
/// ```
///
/// ## Default configuration
///
/// The default configuration is the same as that of `crate::Session`.
///
/// | Field | Default Value |
/// |-------|---------------|
/// |`next_outgoing_id`| 0 |
/// |`incoming_window`| [`crate::session::DEFAULT_WINDOW`] |
/// |`outgoing_window`| [`crate::session::DEFAULT_WINDOW`] |
/// |`handle_max`| `u32::MAX` |
/// |`offered_capabilities` | `None` |
/// |`desired_capabilities`| `None` |
/// |`Properties`| `None` |
///
/// # Customize the acceptor
///
/// The acceptor can be customized using the builder pattern or by modifying the field
/// directly after the acceptor is built.
///
/// ```rust
/// use crate::acceptor::SessionAcceptor;
///
/// let session_acceptor = SessionAcceptor::builder()
///     .handle_max(16)
///     .build();
/// ```
#[derive(Debug)]
pub struct SessionAcceptor(pub SessionBuilder);

impl Default for SessionAcceptor {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl SessionAcceptor {
    /// Creates a new acceptor with default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new builder for [`SessionAcceptor`]
    pub fn builder() -> Builder<Self, Initialized> {
        Builder::<Self, Initialized>::new()
    }

    #[cfg(not(feature = "transaction"))]
    async fn launch_listener_session_engine<R>(
        &self,
        listener_session: ListenerSession,
        _control_link_outgoing: &mpsc::Sender<LinkFrame>,
        connection: &crate::connection::ConnectionHandle<R>,
        _session_control_tx: &mpsc::Sender<SessionControl>,
        session_control_rx: mpsc::Receiver<SessionControl>,
        incoming: mpsc::Receiver<SessionFrame>,
        outgoing_link_frames: mpsc::Receiver<LinkFrame>,
    ) -> Result<JoinHandle<Result<(), Error>>, BeginError> {
        let engine = SessionEngine::begin_listener_session(
            connection.control.clone(),
            listener_session,
            session_control_rx,
            incoming,
            connection.outgoing.clone(),
            outgoing_link_frames,
        )
        .await?;
        Ok(engine.spawn())
    }

    #[cfg(feature = "transaction")]
    async fn launch_listener_session_engine<R>(
        &self,
        listener_session: ListenerSession,
        control_link_outgoing: &mpsc::Sender<LinkFrame>,
        connection: &crate::connection::ConnectionHandle<R>,
        session_control_tx: &mpsc::Sender<SessionControl>,
        session_control_rx: mpsc::Receiver<SessionControl>,
        incoming: mpsc::Receiver<SessionFrame>,
        outgoing_link_frames: mpsc::Receiver<LinkFrame>,
    ) -> Result<JoinHandle<Result<(), Error>>, BeginError> {
        match self.0.control_link_acceptor.clone() {
            Some(control_link_acceptor) => {
                let txn_manager =
                    TransactionManager::new(control_link_outgoing.clone(), control_link_acceptor);
                let listener_session = TxnSession {
                    control: session_control_tx.clone(),
                    session: listener_session,
                    txn_manager,
                };

                let engine = SessionEngine::begin_listener_session(
                    connection.control.clone(),
                    listener_session,
                    session_control_rx,
                    incoming,
                    connection.outgoing.clone(),
                    outgoing_link_frames,
                )
                .await?;
                Ok(engine.spawn())
            }
            None => {
                let engine = SessionEngine::begin_listener_session(
                    connection.control.clone(),
                    listener_session,
                    session_control_rx,
                    incoming,
                    connection.outgoing.clone(),
                    outgoing_link_frames,
                )
                .await?;
                Ok(engine.spawn())
            }
        }
    }

    /// Accept an incoming session
    pub async fn accept_incoming_session(
        &self,
        incoming_session: IncomingSession,
        connection: &mut ListenerConnectionHandle,
    ) -> Result<ListenerSessionHandle, BeginError> {
        let local_state = SessionState::Unmapped;
        let (session_control_tx, session_control_rx) =
            mpsc::channel::<SessionControl>(DEFAULT_SESSION_CONTROL_BUFFER_SIZE);
        let (incoming_tx, incoming_rx) = mpsc::channel(self.0.buffer_size);
        let (outgoing_tx, outgoing_rx) = mpsc::channel(self.0.buffer_size);
        let (link_listener_tx, link_listener_rx) = mpsc::channel(self.0.buffer_size);

        // create session in connection::Engine
        let outgoing_channel = match connection.allocate_session(incoming_tx).await {
            Ok(channel) => channel,
            Err(error) => match error {
                AllocSessionError::IllegalState => return Err(BeginError::IllegalConnectionState),
                AllocSessionError::ChannelMaxReached => {
                    // A peer that receives a channel number outside the supported range MUST close the connection
                    // with the framing-error error-code
                    let error = definitions::Error::new(
                        ConnectionError::FramingError,
                        "Exceeding channel-max".to_string(),
                        None,
                    );
                    connection
                        .control
                        .send(ConnectionControl::Close(Some(error)))
                        .await
                        .map_err(|_| BeginError::IllegalConnectionState)?;

                    return Err(BeginError::LocalChannelMaxReached);
                }
            },
        };
        let mut session = self.0.clone().into_session(outgoing_channel, local_state);
        session.on_incoming_begin(
            IncomingChannel(incoming_session.channel),
            incoming_session.begin,
        )?;

        let listener_session = ListenerSession {
            session,
            link_listener: link_listener_tx,
        };

        let engine_handle = self
            .launch_listener_session_engine(
                listener_session,
                &outgoing_tx,
                connection,
                &session_control_tx,
                session_control_rx,
                incoming_rx,
                outgoing_rx,
            )
            .await?;

        let handle = SessionHandle {
            control: session_control_tx,
            engine_handle,
            outgoing: outgoing_tx,
            link_listener: link_listener_rx,
        };
        Ok(handle)
    }

    /// Waits for incoming session'e Begin performative and then accepts an incoming session
    #[instrument]
    pub async fn accept(
        &self,
        connection: &mut ListenerConnectionHandle,
    ) -> Result<ListenerSessionHandle, BeginError> {
        let incoming_session = connection
            .next_incoming_session()
            .await
            .ok_or(BeginError::IllegalConnectionState)?;
        self.accept_incoming_session(incoming_session, connection)
            .await
    }
}

impl<S> SessionEngine<S>
where
    S: ListenerSessionEndpoint + endpoint::SessionEndpoint,
    BeginError: From<S::BeginError>,
{
    pub async fn begin_listener_session(
        conn_control: mpsc::Sender<ConnectionControl>,
        session: S,
        control: mpsc::Receiver<SessionControl>,
        incoming: mpsc::Receiver<SessionIncomingItem>,
        outgoing: mpsc::Sender<SessionFrame>,
        outgoing_link_frames: mpsc::Receiver<LinkFrame>,
    ) -> Result<Self, BeginError> {
        tracing::trace!("Instantiating session engine");
        let mut engine = Self {
            conn_control,
            session,
            control,
            incoming,
            outgoing,
            outgoing_link_frames,
        };

        // send a begin
        engine.session.send_begin(&mut engine.outgoing).await?;
        Ok(engine)
    }
}

/// A session on the listener side
#[derive(Debug)]
pub struct ListenerSession {
    pub(crate) session: session::Session,
    pub(crate) link_listener: mpsc::Sender<Attach>,
}

impl endpoint::SessionExt for ListenerSession {
    // fn control(&self) -> &mpsc::Sender<SessionControl> {
    //     &self.session.control
    // }
}

#[async_trait]
impl endpoint::Session for ListenerSession {
    type AllocError = <session::Session as endpoint::Session>::AllocError;
    type BeginError = <session::Session as endpoint::Session>::BeginError;
    type EndError = <session::Session as endpoint::Session>::EndError;
    type Error = <session::Session as endpoint::Session>::Error;

    type State = <session::Session as endpoint::Session>::State;

    fn local_state(&self) -> &Self::State {
        self.session.local_state()
    }

    fn local_state_mut(&mut self) -> &mut Self::State {
        self.session.local_state_mut()
    }

    fn outgoing_channel(&self) -> OutgoingChannel {
        self.session.outgoing_channel()
    }

    fn allocate_link(
        &mut self,
        link_name: String,
        link_handle: Option<LinkRelay<()>>,
    ) -> Result<OutputHandle, Self::AllocError> {
        self.session.allocate_link(link_name, link_handle)
    }

    fn allocate_incoming_link(
        &mut self,
        link_name: String,
        link_handle: LinkRelay<()>,
        input_handle: InputHandle,
    ) -> Result<OutputHandle, Self::AllocError> {
        self.session
            .allocate_incoming_link(link_name, link_handle, input_handle)
    }

    fn deallocate_link(&mut self, output_handle: OutputHandle) {
        self.session.deallocate_link(output_handle)
    }

    fn on_incoming_begin(
        &mut self,
        channel: IncomingChannel,
        begin: Begin,
    ) -> Result<(), Self::BeginError> {
        self.session.on_incoming_begin(channel, begin)
    }

    async fn on_incoming_attach(&mut self, attach: Attach) -> Result<(), Self::Error> {
        match self.session.link_by_name.get_mut(&attach.name) {
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

                    let input_handle = attach.handle.clone().into(); // handle is just a wrapper around u32
                    relay
                        .send(LinkFrame::Attach(attach))
                        .await
                        .map_err(|_| Error::UnattachedHandle)?;
                    self.session
                        .link_by_input_handle
                        .insert(input_handle, relay);
                    Ok(())
                }
                None => {
                    // TODO: Resuming link
                    self.link_listener.send(attach).await.map_err(|_| {
                        // SessionHandle must have been dropped, then treat it as if the acceptor doesn't exist
                        Error::HandleInUse
                    })
                }
            },
            None => {
                // If no such terminus exists, the application MAY
                // choose to create one using the properties supplied by the
                // remote link endpoint. The link endpoint is then mapped
                // to an unused handle, and an attach frame is issued carrying
                // the state of the newly created endpoint.

                self.link_listener.send(attach).await.map_err(|_| {
                    // SessionHandle must have been dropped, then treat it as if the acceptor doesn't exist
                    Error::UnattachedHandle
                })
            }
        }
    }

    async fn on_incoming_flow(&mut self, flow: Flow) -> Result<Option<LinkFlow>, Self::Error> {
        self.session.on_incoming_flow(flow).await
    }

    async fn on_incoming_transfer(
        &mut self,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<Option<Disposition>, Self::Error> {
        self.session.on_incoming_transfer(transfer, payload).await
    }

    async fn on_incoming_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<Option<Vec<Disposition>>, Self::Error> {
        self.session.on_incoming_disposition(disposition).await
    }

    async fn on_incoming_detach(&mut self, detach: Detach) -> Result<(), Self::Error> {
        self.session.on_incoming_detach(detach).await
    }

    async fn on_incoming_end(
        &mut self,
        channel: IncomingChannel,
        end: End,
    ) -> Result<(), Self::EndError> {
        self.session.on_incoming_end(channel, end).await
    }

    // Handling SessionFrames
    async fn send_begin(
        &mut self,
        writer: &mpsc::Sender<SessionFrame>,
    ) -> Result<(), Self::BeginError> {
        self.session.send_begin(writer).await
    }

    async fn send_end(
        &mut self,
        writer: &mpsc::Sender<SessionFrame>,
        error: Option<definitions::Error>,
    ) -> Result<(), Self::EndError> {
        self.session.send_end(writer, error).await
    }

    // Intercepting LinkFrames
    fn on_outgoing_attach(&mut self, attach: Attach) -> Result<SessionFrame, Self::Error> {
        self.session.on_outgoing_attach(attach)
    }

    fn on_outgoing_flow(&mut self, flow: LinkFlow) -> Result<SessionFrame, Self::Error> {
        self.session.on_outgoing_flow(flow)
    }

    fn on_outgoing_transfer(
        &mut self,
        input_handle: InputHandle,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<SessionFrame, Self::Error> {
        self.session
            .on_outgoing_transfer(input_handle, transfer, payload)
    }

    fn on_outgoing_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<SessionFrame, Self::Error> {
        self.session.on_outgoing_disposition(disposition)
    }

    fn on_outgoing_detach(&mut self, detach: Detach) -> SessionFrame {
        self.session.on_outgoing_detach(detach)
    }
}

#[cfg(feature = "transaction")]
impl endpoint::HandleDeclare for ListenerSession {
    // This should be unreachable, but an error is probably a better way
    fn allocate_transaction_id(
        &mut self,
    ) -> Result<fe2o3_amqp_types::transaction::TransactionId, AllocTxnIdError> {
        Err(AllocTxnIdError::NotImplemented)
    }
}

#[cfg(feature = "transaction")]
#[async_trait]
impl endpoint::HandleDischarge for ListenerSession {
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
