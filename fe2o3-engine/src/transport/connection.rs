use std::collections::HashMap;

pub use crate::transport::transport::Transport;
use tokio::sync::mpsc::{Sender, Receiver};

use super::{amqp::Frame, session::{SessionHandle, SessionId}};

pub type ChannelId = u16;

pub struct Connection<Io> {
    transport: Transport<Io>,

    local_sessions: HashMap<ChannelId, SessionHandle>,

}