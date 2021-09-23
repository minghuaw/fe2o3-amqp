use fe2o3_types::definitions::SequenceNo;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::transport::connection;

use super::{SessionFrame, SessionState};

pub(crate) struct SessionLocalOption {
    outgoing: Sender<SessionFrame>,

    // local states
    local_channel: u16,
    local_state: SessionState,

    next_incoming_id: SequenceNo,
    incoming_window: SequenceNo,
    next_outgoing_id: SequenceNo,
    outgoing_window: SequenceNo,
}

// pub(crate) MuxHandle {
    
// }

/// Mux has to be started from the Connection's Mux
pub(crate) struct Mux {
    // local states
    outgoing: Sender<SessionFrame>,
    local_channel: u16,
    local_state: SessionState,
    
    next_incoming_id: SequenceNo,
    incoming_window: SequenceNo,
    next_outgoing_id: SequenceNo,
    outgoing_window: SequenceNo,
    
    // remote states
    incoming: Receiver<SessionFrame>,
    remote_channel: Option<u16>,

    remote_incoming_window: SequenceNo,
    remote_outgoing_window: SequenceNo,
}