use std::{collections::{BTreeMap, HashMap}, convert::TryInto, marker::{self, PhantomData}, sync::Arc};

use crate::error::EngineError;
pub use crate::transport::Transport;
use fe2o3_types::{definitions::Milliseconds, performatives::{ChannelMax, MaxFrameSize, Open}};
use tokio::{net::TcpStream, sync::mpsc::{Sender, Receiver}};
use url::Url;

use self::{builder::WithoutContainerId, mux::MuxHandle};

use super::{amqp::{Frame, FrameBody}, protocol_header::ProtocolHeader, session::SessionHandle};

mod builder;
pub use builder::{Builder};
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

#[derive(Debug, Clone,)]
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
pub struct Connection {
    // FIXME: is this really needed?
    // local_open: Arc<Open>, // parameters should be set using the builder and not change before reconnect
    mux: MuxHandle,
}

impl Connection {
    pub async fn open(
        container_id: String,
        max_frame_size: impl Into<MaxFrameSize>,
        channel_max: impl Into<ChannelMax>,
        url: impl TryInto<Url, Error=url::ParseError>
    ) -> Result<Connection, EngineError> {
        Connection::builder()
            .container_id(container_id)
            .max_frame_size(max_frame_size)
            .channel_max(channel_max)
            .open(url).await
    }

    pub fn mux(&self) -> &MuxHandle {
        &self.mux
    }

    pub fn mux_mut(&mut self) -> &mut MuxHandle {
        &mut self.mux
    }

    pub fn builder() -> Builder<WithoutContainerId> {
        Builder::new()
    }
}

impl From<MuxHandle> for Connection {
    fn from(mux: MuxHandle) -> Self {
        Self { mux }
    }
}