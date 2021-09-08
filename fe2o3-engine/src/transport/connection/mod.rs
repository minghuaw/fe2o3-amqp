use std::collections::{BTreeMap, HashMap};

use crate::error::EngineError;
pub use crate::transport::transport::Transport;
use fe2o3_types::{definitions::Milliseconds, performatives::{ChannelMax, MaxFrameSize, Open}};
use tokio::{net::TcpStream, sync::mpsc::{Sender, Receiver}};
use url::Url;

use super::{amqp::{Frame, FrameBody}, protocol_header::ProtocolHeader, session::SessionHandle};

mod builder;
pub use builder::*;
pub mod mux;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct InChanId(pub u16);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutChanId(pub u16);

impl From<u16> for OutChanId {
    fn from(val: u16) -> Self {
        Self(val)
    }
}

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

// brw can still be used as background task
// The broker message can be 
// ```rust 
// enum Message {
    // Incoming(Frame),
    // Outgoing(Frame)
// }
// ```
pub struct Connection<Io> {
    // The underlying transport
    transport: Transport<Io>,

    // local state
    open: Open,
    state: ConnectionState,
    max_frame_size: MaxFrameSize,
    channel_max: ChannelMax,
    idle_time_outs: Milliseconds,

    local_sessions: BTreeMap<InChanId, SessionHandle>, // TODO: consider removing this

    // remote state
    remote_open: Open,
    remote_proto_header: ProtocolHeader,
    remote_state: ConnectionState, // state is infered not directly communicated
}

impl Connection<TcpStream> {
    // pub async fn connect(&mut self, url: Url) -> Result<Self, EngineError> {
    //     match url.scheme() {
    //         "amqp" => {
    //             let addr = url.socket_addrs(|| Some(fe2o3_types::definitions::PORT))?;
    //             let mut stream = TcpStream::connect(&*addr).await?;

    //             // Negotiate and then bind
    //             let remote_header = Transport::negotiate(&mut stream, ProtocolHeader::amqp()).await?;
    //             let transport = Transport::bind(stream)?;

    //             // Send Open frame
    //             let open
    //             todo!()
    //         },
    //         "amqps" => {
    //             todo!()
    //         }
    //         _ => {
    //             return Err(EngineError::Message("Invalid Url scheme"))
    //         }
    //     }
    // }
}