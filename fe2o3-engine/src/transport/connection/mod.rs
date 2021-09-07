use std::collections::{BTreeMap, HashMap};

pub use crate::transport::transport::Transport;
use fe2o3_types::{definitions::Milliseconds, performatives::{ChannelMax, MaxFrameSize, Open}};
use tokio::sync::mpsc::{Sender, Receiver};

use super::{amqp::{Frame, FrameBody}, session::SessionHandle};

mod builder;
pub use builder::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct InChanId(pub u16);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutChanId(pub u16);

pub enum ConnectionState {
    Start,
    
    HeaderReceived,

    HeaderSent,

    HeaderExchange,

    OpenPipe,

    OpenClosePipe,

    OpenReceived,

    OpenSent,

    ClosePipe,

    Opened,

    CloseReceived,
    
    CloseSent,

    Discarding,

    End,
}

pub struct Connection<Io> {
    // The underlying transport
    transport: Transport<Io>,

    // local state
    open: Open,
    state: ConnectionState,
    max_frame_size: MaxFrameSize,
    channel_max: ChannelMax,
    idle_time_outs: Milliseconds,

    local_sessions: BTreeMap<InChanId, SessionHandle>,

    // remote state
    remote_open: Open,
    remote_state: ConnectionState, // state is infered not directly communicated
}