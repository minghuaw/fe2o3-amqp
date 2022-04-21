//! Session Listener

// /// Listener for incoming session
// #[derive(Debug)]
// pub struct SessionListener {}

use std::io;

use async_trait::async_trait;
use fe2o3_amqp_types::{
    definitions::{self, AmqpError, Handle, SessionError},
    performatives::{Attach, Begin, Detach, Disposition, End, Flow, Transfer},
    states::SessionState,
};
use futures_util::Sink;
use tokio::sync::mpsc;
use tokio_util::sync::PollSender;
use tracing::instrument;

use crate::{
    connection::engine::SessionId,
    control::{ConnectionControl, SessionControl},
    endpoint::{self, LinkFlow, Session},
    link::{LinkFrame, LinkHandle},
    session::{
        self,
        engine::SessionEngine,
        frame::{SessionFrame, SessionIncomingItem},
        Error, SessionHandle, DEFAULT_SESSION_CONTROL_BUFFER_SIZE,
    },
    util::Initialized,
    Payload,
};

use super::{builder::Builder, IncomingSession, ListenerConnectionHandle};

type SessionBuilder = crate::session::Builder;

/// Type alias for listener session handle
pub type ListenerSessionHandle = SessionHandle<mpsc::Receiver<Attach>>;

impl ListenerSessionHandle {
    /// Waits for the next incoming link
    pub async fn next_incoming_attach(&mut self) -> Option<Attach> {
        self.link_listener.recv().await
    }
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

    /// Accept an incoming session
    pub async fn accept_incoming_session(
        &self,
        incoming_session: IncomingSession,
        connection: &mut ListenerConnectionHandle,
    ) -> Result<ListenerSessionHandle, Error> {
        let local_state = SessionState::Unmapped;
        let (session_control_tx, session_control_rx) =
            mpsc::channel::<SessionControl>(DEFAULT_SESSION_CONTROL_BUFFER_SIZE);
        let (incoming_tx, incoming_rx) = mpsc::channel(self.0.buffer_size);
        let (outgoing_tx, outgoing_rx) = mpsc::channel(self.0.buffer_size);
        let (link_listener_tx, link_listener_rx) = mpsc::channel(self.0.buffer_size);

        // create session in connection::Engine
        let (outgoing_channel, session_id) = connection.allocate_session(incoming_tx).await?; // AllocSessionError
        let mut session =
            self.0
                .clone()
                .into_session(session_control_tx.clone(), outgoing_channel, local_state);
        session.on_incoming_begin(incoming_session.channel, incoming_session.begin)?;

        let listener_session = ListenerSession {
            session,
            link_listener: link_listener_tx,
        };
        let engine = SessionEngine::<ListenerSession>::begin(
            connection.control.clone(),
            listener_session,
            session_id,
            session_control_rx,
            incoming_rx,
            PollSender::new(connection.outgoing.clone()),
            outgoing_rx,
        )
        .await?;
        let engine_handle = engine.spawn();
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
    ) -> Result<ListenerSessionHandle, Error> {
        let incoming_session = connection.next_incoming_session().await.ok_or_else(|| {
            io::Error::new(io::ErrorKind::Other, "Connection must have been dropped")
        })?;
        self.accept_incoming_session(incoming_session, connection)
            .await
    }
}

impl SessionEngine<ListenerSession> {
    pub async fn begin(
        conn: mpsc::Sender<ConnectionControl>,
        session: ListenerSession,
        session_id: SessionId,
        control: mpsc::Receiver<SessionControl>,
        incoming: mpsc::Receiver<SessionIncomingItem>,
        outgoing: PollSender<SessionFrame>,
        outgoing_link_frames: mpsc::Receiver<LinkFrame>,
    ) -> Result<Self, Error> {
        tracing::trace!("Instantiating session engine");
        let mut engine = Self {
            conn,
            session,
            session_id,
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

#[async_trait]
impl endpoint::Session for ListenerSession {
    type AllocError = <session::Session as endpoint::Session>::AllocError;

    type Error = <session::Session as endpoint::Session>::Error;

    type State = <session::Session as endpoint::Session>::State;

    type LinkHandle = <session::Session as endpoint::Session>::LinkHandle;

    fn local_state(&self) -> &Self::State {
        self.session.local_state()
    }

    fn local_state_mut(&mut self) -> &mut Self::State {
        self.session.local_state_mut()
    }

    fn outgoing_channel(&self) -> u16 {
        self.session.outgoing_channel()
    }

    fn allocate_link(
        &mut self,
        link_name: String,
        link_handle: Self::LinkHandle,
    ) -> Result<fe2o3_amqp_types::definitions::Handle, Self::AllocError> {
        self.session.allocate_link(link_name, link_handle)
    }

    fn allocate_incoming_link(
        &mut self,
        link_name: String,
        link_handle: LinkHandle,
        input_handle: Handle,
    ) -> Result<Handle, Self::AllocError> {
        self.session
            .allocate_incoming_link(link_name, link_handle, input_handle)
    }

    fn deallocate_link(&mut self, link_name: String) {
        self.session.deallocate_link(link_name)
    }

    fn on_incoming_begin(&mut self, channel: u16, begin: Begin) -> Result<(), Self::Error> {
        self.session.on_incoming_begin(channel, begin)
    }

    async fn on_incoming_attach(
        &mut self,
        _channel: u16,
        attach: Attach,
    ) -> Result<(), Self::Error> {
        // Look up link handle by link name
        match self.session.link_by_name.get(&attach.name) {
            Some(output_handle) => match self.session.local_links.get_mut(output_handle.0 as usize)
            {
                Some(link) => {
                    // Only Sender need to update the receiver settle mode
                    // because the sender needs to echo a disposition if
                    // rcv-settle-mode is 1
                    if let LinkHandle::Sender {
                        receiver_settle_mode,
                        ..
                    } = link
                    {
                        *receiver_settle_mode = attach.rcv_settle_mode.clone();
                    }

                    let input_handle = attach.handle.clone(); // handle is just a wrapper around u32
                    self.session
                        .link_by_input_handle
                        .insert(input_handle, output_handle.clone());
                    match link.send(LinkFrame::Attach(attach)).await {
                        Ok(_) => Ok(()),
                        Err(_) => {
                            // TODO: how should this error be handled?
                            // End with UnattachedHandle?
                            return Err(Error::session_error(SessionError::UnattachedHandle, None));
                            // End session with unattached handle?
                        }
                    }
                }
                None => {
                    // TODO: Resuming link
                    return Err(Error::amqp_error(
                        AmqpError::NotImplemented,
                        "Link resumption is not supported yet".to_string(),
                    ));
                }
            },
            None => {
                // If no such terminus exists, the application MAY
                // choose to create one using the properties supplied by the
                // remote link endpoint. The link endpoint is then mapped
                // to an unused handle, and an attach frame is issued carrying
                // the state of the newly created endpoint.

                self.link_listener.send(attach).await.map_err(|_| {
                    // SessionHandle must have been dropped
                    Error::amqp_error(
                        AmqpError::IllegalState,
                        Some("Listener session handle must have been dropped".to_string()),
                    )
                })?;
                Ok(())
            }
        }
    }

    async fn on_incoming_flow(&mut self, channel: u16, flow: Flow) -> Result<(), Self::Error> {
        self.session.on_incoming_flow(channel, flow).await
    }

    async fn on_incoming_transfer(
        &mut self,
        channel: u16,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<(), Self::Error> {
        self.session
            .on_incoming_transfer(channel, transfer, payload)
            .await
    }

    async fn on_incoming_disposition(
        &mut self,
        channel: u16,
        disposition: Disposition,
    ) -> Result<(), Self::Error> {
        self.session
            .on_incoming_disposition(channel, disposition)
            .await
    }

    async fn on_incoming_detach(
        &mut self,
        channel: u16,
        detach: Detach,
    ) -> Result<(), Self::Error> {
        self.session.on_incoming_detach(channel, detach).await
    }

    async fn on_incoming_end(&mut self, channel: u16, end: End) -> Result<(), Self::Error> {
        self.session.on_incoming_end(channel, end).await
    }

    // Handling SessionFrames
    async fn send_begin<W>(&mut self, writer: &mut W) -> Result<(), Self::Error>
    where
        W: Sink<SessionFrame> + Send + Unpin,
    {
        self.session.send_begin(writer).await
    }

    async fn send_end<W>(
        &mut self,
        writer: &mut W,
        error: Option<definitions::Error>,
    ) -> Result<(), Self::Error>
    where
        W: Sink<SessionFrame> + Send + Unpin,
    {
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
        transfer: Transfer,
        payload: Payload,
    ) -> Result<SessionFrame, Self::Error> {
        self.session.on_outgoing_transfer(transfer, payload)
    }

    fn on_outgoing_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<SessionFrame, Self::Error> {
        self.session.on_outgoing_disposition(disposition)
    }

    fn on_outgoing_detach(&mut self, detach: Detach) -> Result<SessionFrame, Self::Error> {
        self.session.on_outgoing_detach(detach)
    }
}
