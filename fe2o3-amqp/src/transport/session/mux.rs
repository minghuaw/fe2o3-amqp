use serde_amqp::primitives::{Symbol};
use fe2o3_amqp_types::{definitions::{Error, Fields, Handle, SequenceNo, TransferNumber}, performatives::{Begin, End}};
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
};

use crate::{error::EngineError, transport::{connection::{OutgoingChannelId, DEFAULT_CONTROL_CHAN_BUF}}};

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
    #[inline]
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

    #[inline]
    async fn recv_begin(&mut self) -> Result<&SessionState, EngineError> {
        println!(">>> Debug: recv_begin");
        let frame = match self.incoming.recv().await {
            Some(frame) => frame?,
            None => return Err(EngineError::Message("Unexpected Eof of SessionFrame")) // TODO: send back error?
        };
        let remote_begin = match frame.body {
            SessionFrameBody::Begin{performative} => performative,
            _ => return Err(EngineError::Message("Expecting Begin"))
        };
        
        self.handle_begin_recv(remote_begin).await
    } 

    #[inline]
    async fn handle_begin_recv(&mut self, remote_begin: Begin) -> Result<&SessionState, EngineError> {
        println!(">>> Debug: handle_begin_recv()");
        match &self.local_state {
            SessionState::Unmapped => self.local_state = SessionState::BeginReceived,
            SessionState::BeginSent => self.local_state = SessionState::Mapped,
            _ => return Err(EngineError::illegal_state())
        }
        
        self.next_incoming_id = remote_begin.next_outgoing_id;
        self.remote_incoming_window = remote_begin.incoming_window;
        self.remote_outgoing_window = remote_begin.outgoing_window;

        // TODO: is there anything else to check?
        Ok(&self.local_state)
    }

    #[inline]
    async fn handle_end_send(&mut self, local_error: Option<Error>) -> Result<&SessionState, EngineError> {
        println!(">>> Debug: handle_end_send()");
        match &self.local_state {
            SessionState::Mapped => {
                if local_error.is_some() {
                    self.local_state = SessionState::Discarding;
                } else {
                    self.local_state = SessionState::EndSent;
                }
            }
            SessionState::EndReceived => self.local_state = SessionState::Unmapped,
            _ => return Err(EngineError::illegal_state())
        }

        let frame = SessionFrame::new(
            self.local_channel.0, 
            SessionFrameBody::end(End { error: local_error })
        );
        self.outgoing.send(frame).await?;
        Ok(&self.local_state)
    }

    #[inline]
    async fn handle_end_recv(&mut self, _remote_end: End) -> Result<&SessionState, EngineError> {
        println!(">>> Debug: handle_end_recv()");

        match &self.local_state {
            SessionState::Mapped => {
                self.local_state = SessionState::EndReceived;
                // TODO: respond with an end?
                self.handle_end_send(None).await?;
            },
            SessionState::Discarding | SessionState::EndSent => self.local_state = SessionState::Unmapped,
            _ => return Err(EngineError::illegal_state())
        }
        Ok(&self.local_state)
    }

    #[inline]
    async fn handle_incoming(&mut self, item: Result<SessionFrame, EngineError>) -> Result<&SessionState, EngineError> {
        println!(">>> Debug: session/mux.rs handle_incoming()");
        let frame = item?;
        // local state check should be checked in each sub-handlers
        let remote_channel = frame.channel;
        match frame.body {
            SessionFrameBody::Begin{performative} => self.handle_begin_recv(performative).await,
            SessionFrameBody::End{performative} => self.handle_end_recv(performative).await,
            _ => todo!()
        }
    }

    #[inline]
    async fn handle_error(&mut self, error: EngineError) -> &SessionState {
        todo!()
    }

    #[inline]
    async fn mux_loop(mut self) -> Result<(), EngineError> {
        loop {
            let result = tokio::select! {
                // local controls
                control = self.control.recv() => {
                    match control {
                        Some(control) => match control {
                            SessionMuxControl::End => {
                                self.handle_end_send(None).await
                            }
                        },
                        None => {
                            // TODO: the session should probably stop when control is dropped
                            todo!()
                        }
                    }
                },
                // incoming session frames
                next = self.incoming.recv() => {
                    println!("session mux recved a frame");
                    match next {
                        Some(item) => self.handle_incoming(item).await,
                        // TODO: "Sessions end automatically when the connection is closed or interrupted"
                        None => todo!()
                    }
                }
            };

            let state = match result {
                Ok(state) => state,
                Err(error) => self.handle_error(error).await
            };

            // There is no obligation to retain a session endpoint
            // after it transitions to the UNMAPPED state.
            if let SessionState::Unmapped = state {
                return Ok(())
            }
        }
    }
}
