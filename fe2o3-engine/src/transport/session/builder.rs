use fe2o3_amqp::primitives::{Symbol, UInt};
use fe2o3_types::definitions::{Fields, Handle, TransferNumber};

use crate::{error::EngineError, transport::connection::Connection};

use super::{DEFAULT_WINDOW, Session, SessionHandle};

pub struct Builder {
    pub next_outgoing_id: TransferNumber,
    pub incoming_window: TransferNumber,
    pub outgoing_window: TransferNumber,
    pub handle_max: Handle,
    pub offered_capabilities: Option<Vec<Symbol>>,
    pub desired_capabilities: Option<Vec<Symbol>>,
    pub properties: Option<Fields>,
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
            properties: None
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

    pub async fn begin(&mut self, connection: &mut Connection) -> Result<Session, EngineError> {
        todo!()
    }
}