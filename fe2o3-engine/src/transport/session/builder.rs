use fe2o3_amqp::primitives::{Symbol, UInt};
use fe2o3_types::definitions::{Fields, Handle, TransferNumber};
use tokio::sync::{mpsc, oneshot};

use crate::{error::EngineError, transport::{connection::{ConnMuxControl, Connection}, session::SessionLocalOption}};

use super::{DEFAULT_WINDOW, Session, SessionHandle, mux::DEFAULT_SESSION_MUX_BUFFER_SIZE};

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

    pub async fn begin(&mut self, connection: &mut Connection) -> Result<Session, EngineError> {
        // create a oneshot channel to receive result of session creation
        let (oneshot_tx, oneshot_rx) = oneshot::channel();
        let (control_tx, control_rx) = mpsc::channel(self.buffer_size);

        // send session options and oneshot sender to Connection's Mux
        let option = SessionLocalOption {
            control: control_rx,
            outgoing: connection.session_tx().clone(),
            next_outgoing_id: self.next_outgoing_id,
            incoming_window: self.incoming_window,
            outgoing_window: self.outgoing_window,
            handle_max: self.handle_max.0,
            offered_capabilities: self.offered_capabilities.clone(),
            desired_capabilities: self.desired_capabilities.clone(),
            properties: self.properties.clone()
        };
        let control = ConnMuxControl::NewSession {
            resp: oneshot_tx,
            option
        };
        connection.mux_mut().send(control).await?;

        // .awaiting result
        let handle = oneshot_rx.await
            .map_err(|_| EngineError::Message("RecvError from oneshot::Receiver"))??;

        Ok(Session {
            mux: control_tx,
            handle
        })
    }
}