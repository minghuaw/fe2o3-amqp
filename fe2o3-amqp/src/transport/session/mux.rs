use serde_amqp::primitives::{Symbol, UInt};
use fe2o3_amqp_types::{
    definitions::{Fields, Handle, SequenceNo, TransferNumber},
    performatives::Begin,
};
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task::JoinHandle,
};

use crate::{error::EngineError, transport::{amqp::{Frame, FrameBody}, connection::{self, OutgoingChannelId, DEFAULT_CONTROL_CHAN_BUF}, session::SessionHandle}};

use super::{Session, SessionFrame, SessionFrameBody, SessionState};

pub const DEFAULT_SESSION_MUX_BUFFER_SIZE: usize = u16::MAX as usize;

pub(crate) enum SessionMuxControl {
    End,
}

/// Mux has to be started from the Connection's Mux
pub(crate) struct SessionMux {
    // control
    control: Receiver<SessionMuxControl>,

    // local states
    // A `local_begin` is not used here (unlike in ConnMux)
    // because the following local states are subject to change
    // during the operation.
    outgoing: Sender<SessionFrame>,
    local_channel: OutgoingChannelId,
    local_state: SessionState,

    next_outgoing_id: TransferNumber,
    incoming_window: TransferNumber,
    outgoing_window: TransferNumber,

    handle_max: Handle,

    // remote states
    incoming: Receiver<Result<SessionFrame, EngineError>>,

    // initialize with 0 first and change after receiving the remote Begin
    next_incoming_id: TransferNumber,
    remote_incoming_window: SequenceNo,
    remote_outgoing_window: SequenceNo,

    // capabilities
    offered_capabilities: Option<Vec<Symbol>>,
    desired_capabilities: Option<Vec<Symbol>>,
    properties: Option<Fields>,
}

impl SessionMux {
    // pub fn spawn(
    //     local_state: SessionState,
    //     local_channel: OutgoingChannelId,
    //     incoming: Receiver<Result<SessionFrame, EngineError>>,
    //     outgoing: Sender<SessionFrame>,
    //     next_outgoing_id: TransferNumber,
    //     incoming_window: TransferNumber,
    //     outgoing_window: TransferNumber,
    //     handle_max: Handle,
    //     offered_capabilities: Option<Vec<Symbol>>,
    //     desired_capabilities: Option<Vec<Symbol>>,
    //     properties: Option<Fields>,
    // ) -> Result<Session, EngineError> {
    //     // channels
    //     let (control_tx, control) = mpsc::channel(DEFAULT_CONTROL_CHAN_BUF);

    //     let mux = SessionMux {
    //         control,
    //         outgoing,
    //         local_channel,
    //         local_state,
    //         next_incoming_id: 0, // initialize with 0 and update when remote Begin is received
    //         incoming_window,
    //         next_outgoing_id,
    //         outgoing_window,
    //         handle_max,
    //         incoming,
    //         remote_incoming_window: 0, // initialize with 0 and update when remote Begin is received
    //         remote_outgoing_window: 0, // initialize with 0 and update when remote Begin is received
    //         offered_capabilities,
    //         desired_capabilities,
    //         properties
    //     };

    //     let handle = tokio::spawn(mux.mux_loop());
    //     let session = Session {
    //         mux: control_tx,
    //         handle,
    //     };

    //     // Send begin and wait for begin
    //     todo!();

    //     Ok(session)
    // }

    pub async fn begin(
        local_state: SessionState,
        local_channel: OutgoingChannelId,
        incoming: Receiver<Result<SessionFrame, EngineError>>,
        outgoing: Sender<SessionFrame>,
        next_outgoing_id: TransferNumber,
        incoming_window: TransferNumber,
        outgoing_window: TransferNumber,
        handle_max: Handle,
        offered_capabilities: Option<Vec<Symbol>>,
        desired_capabilities: Option<Vec<Symbol>>,
        properties: Option<Fields>,
    ) -> Result<Session, EngineError> {
        // channels
        let (control_tx, control) = mpsc::channel(DEFAULT_CONTROL_CHAN_BUF);

        let mut mux = SessionMux {
            control,
            outgoing,
            local_channel,
            local_state,
            next_incoming_id: 0, // initialize with 0 and update when remote Begin is received
            incoming_window,
            next_outgoing_id,
            outgoing_window,
            handle_max,
            incoming,
            remote_incoming_window: 0, // initialize with 0 and update when remote Begin is received
            remote_outgoing_window: 0, // initialize with 0 and update when remote Begin is received
            offered_capabilities,
            desired_capabilities,
            properties
        };

        // Send begin and wait for begin
        mux.send_begin().await?;
        mux.recv_begin().await?;

        let handle = tokio::spawn(mux.mux_loop());
        let session = Session {
            mux: control_tx,
            handle,
        };

        Ok(session)
    }
}



/* ----------------------------- private methods ---------------------------- */
impl SessionMux {
    async fn send_begin(&mut self) -> Result<&SessionState, EngineError> {
        println!(">>> Debug: send_begin()");
        let performative = Begin {
            remote_channel: None,
            next_outgoing_id: self.next_outgoing_id,
            incoming_window: self.incoming_window,
            outgoing_window: self.outgoing_window,
            handle_max: self.handle_max.clone(),
            offered_capabilities: self.offered_capabilities.clone(),
            desired_capabilities: self.desired_capabilities.clone(),
            properties: self.properties.clone()
        };

        let frame = SessionFrame::new(
            self.local_channel.0,
            SessionFrameBody::begin(performative)
        );

        match &self.local_state {
            SessionState::Unmapped => {
                self.outgoing.send(frame).await?;
                self.local_state = SessionState::BeginSent;
            },
            SessionState::BeginReceived => {
                self.outgoing.send(frame).await?;
                self.local_state = SessionState::Mapped;
            },
            _ => return Err(EngineError::illegal_state())
        }

        Ok(&self.local_state)
    }

    async fn recv_begin(&mut self) -> Result<&SessionState, EngineError> {
        let frame = match self.incoming.recv().await {
            Some(frame) => frame?,
            None => return Err(EngineError::Message("Unexpected Eof of SessionFrame")) // TODO: send back error?
        };
        let remote_begin = match frame.body {
            SessionFrameBody::Begin{performative} => performative,
            _ => return Err(EngineError::Message("Expecting Begin"))
        };
        
        self.next_incoming_id = remote_begin.next_outgoing_id;
        self.remote_incoming_window = remote_begin.incoming_window;
        self.remote_outgoing_window = remote_begin.outgoing_window;

        // TODO: is there anything else to check?
        Ok(&self.local_state)
    }

    async fn mux_loop(mut self) -> Result<(), EngineError> {
        loop {
            println!(">>> Debug: SessionMux mux_loop");
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
}
