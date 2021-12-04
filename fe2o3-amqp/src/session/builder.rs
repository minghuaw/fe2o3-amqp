use std::collections::BTreeMap;

use fe2o3_amqp_types::definitions::{Fields, Handle, TransferNumber};
use serde_amqp::primitives::{Symbol, UInt};
use slab::Slab;
use tokio::sync::mpsc;
use tokio_util::sync::PollSender;

use crate::{
    connection::{builder::DEFAULT_OUTGOING_BUFFER_SIZE, ConnectionHandle},
    control::SessionControl,
    session::{engine::SessionEngine, SessionState},
    util::Constant,
};

use super::{SessionHandle, Error};

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

    pub fn next_outgoing_id(mut self, value: TransferNumber) -> Self {
        self.next_outgoing_id = value;
        self
    }

    pub fn incoming_window(mut self, value: TransferNumber) -> Self {
        self.incoming_window = value;
        self
    }

    pub fn outgoing_widnow(mut self, value: TransferNumber) -> Self {
        self.outgoing_window = value;
        self
    }

    pub fn handle_max(mut self, value: impl Into<Handle>) -> Self {
        self.handle_max = value.into();
        self
    }

    pub fn add_offered_capabilities(mut self, capability: impl Into<Symbol>) -> Self {
        match &mut self.offered_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.offered_capabilities = Some(vec![capability.into()]),
        }
        self
    }

    pub fn set_offered_capabilities(mut self, capabilities: Vec<Symbol>) -> Self {
        self.offered_capabilities = Some(capabilities);
        self
    }

    pub fn add_desired_capabilities(mut self, capability: impl Into<Symbol>) -> Self {
        match &mut self.desired_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.desired_capabilities = Some(vec![capability.into()]),
        }
        self
    }

    pub fn set_desired_capabilities(mut self, capabilities: Vec<Symbol>) -> Self {
        self.desired_capabilities = Some(capabilities);
        self
    }

    pub fn properties(mut self, properties: Fields) -> Self {
        self.properties = Some(properties);
        self
    }

    pub fn buffer_size(mut self, buffer_size: usize) -> Self {
        self.buffer_size = buffer_size;
        self
    }

    pub async fn begin(self, conn: &mut ConnectionHandle) -> Result<SessionHandle, Error> {
        use super::Session;

        let local_state = SessionState::Unmapped;
        let (session_control_tx, session_control_rx) =
            mpsc::channel::<SessionControl>(DEFAULT_SESSION_CONTROL_BUFFER_SIZE);
        let (incoming_tx, incoming_rx) = mpsc::channel(self.buffer_size);
        let (outgoing_tx, outgoing_rx) = mpsc::channel(DEFAULT_OUTGOING_BUFFER_SIZE);

        // create session in connection::Engine
        let (outgoing_channel, session_id) = conn.allocate_session(incoming_tx).await?;

        let session = Session {
            control: session_control_tx.clone(),
            // session_id,
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

            local_links: Slab::new(),
            link_by_name: BTreeMap::new(),
            link_by_input_handle: BTreeMap::new(),
        };
        let engine = SessionEngine::begin(
            conn.control.clone(),
            session,
            session_id,
            session_control_rx,
            incoming_rx,
            PollSender::new(conn.outgoing.clone()),
            outgoing_rx,
        )
        .await?;

        let handle = engine.spawn();
        let handle = SessionHandle {
            control: session_control_tx,
            handle,
            outgoing: outgoing_tx,
        };
        Ok(handle)
    }
}
