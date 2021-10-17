use serde_amqp::primitives::{Symbol, UInt};
use fe2o3_amqp_types::definitions::{Fields, Handle, TransferNumber};
use tokio::sync::{mpsc};

use crate::{connection::ConnectionHandle, control::{ConnectionControl, SessionControl}, error::EngineError, session::{SessionFrame, SessionState, engine::SessionEngine}};

use super::SessionHandle;

pub const DEFAULT_SESSION_CONTROL_BUFFER_SIZE: usize = 128;
pub const DEFAULT_SESSION_MUX_BUFFER_SIZE: usize = u16::MAX as usize;
/// Default incoming_window and outgoing_window
pub const DEFAULT_WINDOW: UInt = 2048;

pub struct Builder {
    pub next_outgoing_id: TransferNumber,
    pub incoming_window: TransferNumber,
    pub outgoing_window: TransferNumber,
    pub handle_max: Handle,
    pub offered_capabilities: Option<Vec<Symbol>>,
    pub desired_capabilities: Option<Vec<Symbol>>,
    pub properties: Option<Fields>,
    pub buffer_size: usize,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            next_outgoing_id: 0,
            incoming_window: DEFAULT_WINDOW,
            outgoing_window: DEFAULT_WINDOW,
            handle_max: Default::default(),
            offered_capabilities: None,
            desired_capabilities: None,
            properties: None,
            buffer_size: DEFAULT_SESSION_MUX_BUFFER_SIZE,
        }
    }

    pub fn next_outgoing_id(&mut self, value: TransferNumber) -> &mut Self {
        self.next_outgoing_id = value;
        self
    }

    pub fn incoming_window(&mut self, value: TransferNumber) -> &mut Self {
        self.incoming_window = value;
        self
    }

    pub fn outgoing_widnow(&mut self, value: TransferNumber) -> &mut Self {
        self.outgoing_window = value;
        self
    }

    pub fn handle_max(&mut self, value: impl Into<Handle>) -> &mut Self {
        self.handle_max = value.into();
        self
    }

    pub fn add_offered_capabilities(&mut self, capability: impl Into<Symbol>) -> &mut Self {
        match &mut self.offered_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.offered_capabilities = Some(vec![capability.into()]),
        }
        self
    }

    pub fn set_offered_capabilities(&mut self, capabilities: Vec<Symbol>) -> &mut Self {
        self.offered_capabilities = Some(capabilities);
        self
    }

    pub fn add_desired_capabilities(&mut self, capability: impl Into<Symbol>) -> &mut Self {
        match &mut self.desired_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.desired_capabilities = Some(vec![capability.into()]),
        }
        self
    }

    pub fn set_desired_capabilities(&mut self, capabilities: Vec<Symbol>) -> &mut Self {
        self.desired_capabilities = Some(capabilities);
        self
    }

    pub fn properties(&mut self, properties: Fields) -> &mut Self {
        self.properties = Some(properties);
        self
    }

    pub fn buffer_size(&mut self, buffer_size: usize) -> &mut Self {
        self.buffer_size = buffer_size;
        self
    }

    pub async fn begin(&mut self, conn: &mut ConnectionHandle) -> Result<SessionHandle, EngineError> {
        use tokio::sync::oneshot;
        use super::Session;

        let local_state = SessionState::Unmapped;
        let (session_control_tx, session_control_rx) = mpsc::channel::<SessionControl>(DEFAULT_SESSION_CONTROL_BUFFER_SIZE);
        let (oneshot_tx, oneshot_rx) = oneshot::channel::<(u16, usize)>();
        let (incoming_tx, incoming_rx) = mpsc::channel(self.buffer_size);

        // create session in connection::Engine
        conn.control.send(ConnectionControl::CreateSession{
            tx: incoming_tx,
            responder: oneshot_tx
        }).await?;
        let (outgoing_channel, session_id) = oneshot_rx.await
            .map_err(|_| EngineError::Message("Oneshot sender is dropped"))?;

        let session = Session::new(
            session_control_tx.clone(),
            session_id,
            outgoing_channel,
            local_state,
            self.next_outgoing_id,
            self.incoming_window,
            self.outgoing_window,
            self.handle_max.clone(),
            0,
            0,
            0,
            self.offered_capabilities.clone(),
            self.desired_capabilities.clone(),
            self.properties.clone()
        );
        let engine = SessionEngine::begin(session, session_control_rx, incoming_rx, conn.outgoing.clone()).await?;
        let handle = engine.spawn();
        let handle = SessionHandle {
            control: session_control_tx,
            handle
        };
        Ok(handle)
    }
}
