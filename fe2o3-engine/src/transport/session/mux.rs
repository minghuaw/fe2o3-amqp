use std::thread::JoinHandle;

use fe2o3_amqp::primitives::UInt;
use fe2o3_types::definitions::{SequenceNo, TransferNumber};
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::{error::EngineError, transport::connection};

use super::{SessionFrame, SessionState};

pub(crate) struct SessionLocalOption {
    // control
    control: Receiver<SessionMuxControl>,
    
    // local states
    outgoing: Sender<SessionFrame>,
    local_channel: u16,
    local_state: SessionState,

    next_incoming_id: TransferNumber,
    incoming_window: TransferNumber,
    next_outgoing_id: TransferNumber,
    outgoing_window: TransferNumber,

    handle_max: UInt,
}

pub(crate) enum SessionMuxControl {
    End,
}

pub(crate) struct SessionMuxHandle {
    control: Sender<SessionMuxControl>,
    // handle: JoinHandle<Result<(), EngineError>>, // JoinHandle to session mux
}

/// Mux has to be started from the Connection's Mux
pub(crate) struct SessionMux {
    // control
    control: Receiver<SessionMuxControl>,

    // local states
    outgoing: Sender<SessionFrame>,
    local_channel: u16,
    local_state: SessionState,

    next_incoming_id: TransferNumber,
    incoming_window: TransferNumber,
    next_outgoing_id: TransferNumber,
    outgoing_window: TransferNumber,

    handle_max: UInt,

    // remote states
    incoming: Receiver<SessionFrame>,
    // remote_channel: u16,

    remote_incoming_window: SequenceNo,
    remote_outgoing_window: SequenceNo,
}

impl SessionMux {
    pub async fn begin_with_option(
        incoming: Receiver<SessionFrame>,
        // remote_channel: u16,
        remote_incoming_window: SequenceNo,
        remote_outgoing_window: SequenceNo,
        // local options
        local_option: SessionLocalOption,
    ) -> Result<SessionMuxHandle, EngineError> {
        let SessionLocalOption {
            control,
            outgoing,
            local_channel,
            local_state,
            next_incoming_id,
            incoming_window,
            next_outgoing_id,
            outgoing_window,
            handle_max,
        } = local_option;

        let mux = SessionMux {
            outgoing,
            local_channel,
            local_state,
            next_incoming_id,
            incoming_window,
            next_outgoing_id,
            outgoing_window,
            handle_max,

            incoming,
            remote_incoming_window,
            remote_outgoing_window,
            control,
        };

        todo!()
    }

    async fn mux_loop(mut self) -> Result<(), EngineError> {
        todo!()
    }
}
