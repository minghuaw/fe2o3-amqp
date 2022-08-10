//! Session builder

use std::collections::BTreeMap;

use fe2o3_amqp_types::definitions::{Fields, Handle, TransferNumber};
use serde_amqp::primitives::Symbol;
use slab::Slab;
use tokio::{sync::mpsc, task::JoinHandle};

use crate::{
    connection::{AllocSessionError, ConnectionHandle},
    control::SessionControl,
    endpoint::OutgoingChannel,
    link::LinkFrame,
    session::{engine::SessionEngine, SessionState},
    util::Constant,
    Session,
};

#[cfg(all(feature = "transaction", feature = "acceptor"))]
use crate::transaction::{
    coordinator::ControlLinkAcceptor, manager::TransactionManager, session::TxnSession,
};

use super::{
    error::{SessionBeginError, SessionErrorKind},
    frame::SessionFrame,
    SessionHandle, DEFAULT_WINDOW,
};

pub(crate) const DEFAULT_SESSION_CONTROL_BUFFER_SIZE: usize = 128;
pub(crate) const DEFAULT_SESSION_MUX_BUFFER_SIZE: usize = u16::MAX as usize;

/// Builder for [`crate::Session`]
#[derive(Debug, Clone)]
pub struct Builder {
    /// The transfer-id of the first transfer id the sender will send
    pub next_outgoing_id: TransferNumber,

    /// The initial incoming-window of the sender
    pub incoming_window: TransferNumber,

    /// The initial outgoing-window of the sender
    pub outgoing_window: TransferNumber,

    /// The maximum handle value that can be used on the session
    pub handle_max: Handle,

    /// The extension capabilities the sender supports
    pub offered_capabilities: Option<Vec<Symbol>>,

    /// The extension capabilities the sender can use if the receiver supports them
    pub desired_capabilities: Option<Vec<Symbol>>,

    /// Session properties
    pub properties: Option<Fields>,

    /// Buffer size of the underlying [`tokio::sync::mpsc::channel`]
    /// that are used by links attached to the session
    pub buffer_size: usize,

    /// Acceptor for incoming transaction control links
    #[cfg(all(feature = "transaction", feature = "acceptor"))]
    pub(crate) control_link_acceptor: Option<ControlLinkAcceptor>,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            next_outgoing_id: 0,
            incoming_window: DEFAULT_WINDOW,
            outgoing_window: DEFAULT_WINDOW,
            handle_max: Default::default(),
            offered_capabilities: None,
            desired_capabilities: None,
            properties: None,
            buffer_size: DEFAULT_SESSION_MUX_BUFFER_SIZE,

            #[cfg(all(feature = "transaction", feature = "acceptor"))]
            control_link_acceptor: None,
        }
    }
}

impl Builder {
    /// Creates a new builder for [`crate::Session`]
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn into_session(
        self,
        control: mpsc::Sender<SessionControl>,
        outgoing_channel: OutgoingChannel,
        local_state: SessionState,
    ) -> Session {
        Session {
            control,
            outgoing_channel,
            local_state,
            initial_outgoing_id: Constant::new(self.next_outgoing_id),
            next_outgoing_id: self.next_outgoing_id,
            incoming_window: self.incoming_window,
            outgoing_window: self.outgoing_window,
            handle_max: self.handle_max,
            incoming_channel: None,
            next_incoming_id: 0,
            remote_incoming_window: 0,
            remote_outgoing_window: 0,
            offered_capabilities: self.offered_capabilities,
            desired_capabilities: self.desired_capabilities,
            properties: self.properties,

            link_name_by_output_handle: Slab::new(),
            link_by_name: BTreeMap::new(),
            link_by_input_handle: BTreeMap::new(),
            delivery_tag_by_id: BTreeMap::new(),
        }
    }

    #[cfg(all(feature = "transaction", feature = "acceptor"))]
    pub(crate) fn into_txn_session(
        self,
        control: mpsc::Sender<SessionControl>,
        outgoing: mpsc::Sender<LinkFrame>,
        outgoing_channel: OutgoingChannel,
        control_link_acceptor: ControlLinkAcceptor,
        local_state: SessionState,
    ) -> TxnSession<Session> {
        let txn_manager = TransactionManager::new(outgoing, control_link_acceptor);
        let session = Session {
            control,
            outgoing_channel,
            local_state,
            initial_outgoing_id: Constant::new(self.next_outgoing_id),
            next_outgoing_id: self.next_outgoing_id,
            incoming_window: self.incoming_window,
            outgoing_window: self.outgoing_window,
            handle_max: self.handle_max,
            incoming_channel: None,
            next_incoming_id: 0,
            remote_incoming_window: 0,
            remote_outgoing_window: 0,
            offered_capabilities: self.offered_capabilities,
            desired_capabilities: self.desired_capabilities,
            properties: self.properties,

            link_name_by_output_handle: Slab::new(),
            link_by_name: BTreeMap::new(),
            link_by_input_handle: BTreeMap::new(),
            delivery_tag_by_id: BTreeMap::new(),
        };

        TxnSession {
            session,
            txn_manager,
        }
    }

    #[cfg(not(all(feature = "transaction", feature = "acceptor")))]
    async fn launch_client_session_engine<R>(
        self,
        session_control_tx: mpsc::Sender<SessionControl>,
        _outgoing: &mpsc::Sender<LinkFrame>,
        outgoing_channel: OutgoingChannel,
        local_state: SessionState,
        connection: &ConnectionHandle<R>,
        session_control_rx: mpsc::Receiver<SessionControl>,
        incoming: mpsc::Receiver<SessionFrame>,
        outgoing_link_frames: mpsc::Receiver<LinkFrame>,
    ) -> Result<JoinHandle<Result<(), SessionErrorKind>>, SessionBeginError> {
        let session = self.into_session(session_control_tx.clone(), outgoing_channel, local_state);
        let engine = SessionEngine::begin_client_session(
            connection.control.clone(),
            session,
            session_control_rx,
            incoming,
            connection.outgoing.clone(),
            outgoing_link_frames,
        )
        .await?;
        Ok(engine.spawn())
    }

    #[cfg(all(feature = "transaction", feature = "acceptor"))]
    async fn launch_client_session_engine<R>(
        mut self,
        session_control_tx: mpsc::Sender<SessionControl>,
        control_link_outgoing: &mpsc::Sender<LinkFrame>,
        outgoing_channel: OutgoingChannel,
        local_state: SessionState,
        connection: &ConnectionHandle<R>,
        session_control_rx: mpsc::Receiver<SessionControl>,
        incoming: mpsc::Receiver<SessionFrame>,
        outgoing_link_frames: mpsc::Receiver<LinkFrame>,
    ) -> Result<JoinHandle<Result<(), SessionErrorKind>>, SessionBeginError> {
        match self.control_link_acceptor.take() {
            Some(control_link_acceptor) => {
                let session = self.into_txn_session(
                    session_control_tx,
                    control_link_outgoing.clone(),
                    outgoing_channel,
                    control_link_acceptor,
                    local_state,
                );
                let engine = SessionEngine::begin_client_session(
                    connection.control.clone(),
                    session,
                    session_control_rx,
                    incoming,
                    connection.outgoing.clone(),
                    outgoing_link_frames,
                )
                .await?;
                Ok(engine.spawn())
            }
            None => {
                let session =
                    self.into_session(session_control_tx.clone(), outgoing_channel, local_state);
                let engine = SessionEngine::begin_client_session(
                    connection.control.clone(),
                    session,
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

    /// The transfer-id of the first transfer id the sender will send
    pub fn next_outgoing_id(mut self, value: TransferNumber) -> Self {
        self.next_outgoing_id = value;
        self
    }

    /// The initial incoming-window of the sender
    pub fn incoming_window(mut self, value: TransferNumber) -> Self {
        self.incoming_window = value;
        self
    }

    /// The initial outgoing-window of the sender
    pub fn outgoing_widnow(mut self, value: TransferNumber) -> Self {
        self.outgoing_window = value;
        self
    }

    /// The maximum handle value that can be used on the session
    pub fn handle_max(mut self, value: impl Into<Handle>) -> Self {
        self.handle_max = value.into();
        self
    }

    /// Add one extension capabilities the sender supports
    pub fn add_offered_capabilities(mut self, capability: impl Into<Symbol>) -> Self {
        match &mut self.offered_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.offered_capabilities = Some(vec![capability.into()]),
        }
        self
    }

    /// Set the extension capabilities the sender supports
    pub fn set_offered_capabilities(mut self, capabilities: Vec<Symbol>) -> Self {
        self.offered_capabilities = Some(capabilities);
        self
    }

    /// Add one extension capabilities the sender can use if the receiver supports them
    pub fn add_desired_capabilities(mut self, capability: impl Into<Symbol>) -> Self {
        match &mut self.desired_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.desired_capabilities = Some(vec![capability.into()]),
        }
        self
    }

    /// Set the extension capabilities the sender can use if the receiver supports them
    pub fn set_desired_capabilities(mut self, capabilities: Vec<Symbol>) -> Self {
        self.desired_capabilities = Some(capabilities);
        self
    }

    /// Session properties
    pub fn properties(mut self, properties: Fields) -> Self {
        self.properties = Some(properties);
        self
    }

    /// Buffer size of the underlying [`tokio::sync::mpsc::channel`]
    /// that are used by links attached to the session
    pub fn buffer_size(mut self, buffer_size: usize) -> Self {
        self.buffer_size = buffer_size;
        self
    }

    // TODO
    // /// Enable handling remotely initiated control link and transaction by setting the
    // /// `control_link_acceptor` field
    // #[cfg(all(feature = "transaction", feature = "acceptor"))]
    // pub fn control_link_acceptor(
    //     mut self,
    //     control_link_acceptor: impl Into<Option<ControlLinkAcceptor>>,
    // ) -> Self {
    //     self.control_link_acceptor = control_link_acceptor.into();
    //     self
    // }

    /// Begins a new session
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// let session = Session::builder()
    ///     .handle_max(128u32)
    ///     .begin(&mut connection)
    ///     .await.unwrap();
    /// ```
    ///
    pub async fn begin(
        self,
        connection: &mut ConnectionHandle<()>,
    ) -> Result<SessionHandle<()>, SessionBeginError> {
        let local_state = SessionState::Unmapped;
        let (session_control_tx, session_control_rx) =
            mpsc::channel::<SessionControl>(DEFAULT_SESSION_CONTROL_BUFFER_SIZE);
        let (incoming_tx, incoming_rx) = mpsc::channel(self.buffer_size);
        let (outgoing_tx, outgoing_rx) = mpsc::channel(self.buffer_size);

        // create session in connection::Engine
        let outgoing_channel = match connection.allocate_session(incoming_tx).await {
            Ok(channel) => channel,
            Err(alloc_error) => match alloc_error {
                AllocSessionError::IllegalState => {
                    return Err(SessionBeginError::IllegalConnectionState)
                }
                AllocSessionError::ChannelMaxReached => {
                    // Locally initiating session exceeded channel max
                    return Err(SessionBeginError::LocalChannelMaxReached);
                }
            },
        };

        let engine_handle = self
            .launch_client_session_engine(
                session_control_tx.clone(),
                &outgoing_tx,
                outgoing_channel,
                local_state,
                connection,
                session_control_rx,
                incoming_rx,
                outgoing_rx,
            )
            .await?;

        let handle = SessionHandle {
            control: session_control_tx,
            engine_handle,
            outgoing: outgoing_tx,
            link_listener: (),
        };
        Ok(handle)
    }
}
