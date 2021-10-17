//! Controls for Connection, Session, and Link

use std::sync::mpsc::Receiver;

use fe2o3_amqp_types::{definitions::Error};
use tokio::sync::{mpsc::Sender, oneshot};

use crate::session::SessionFrame;

pub enum ConnectionControl {
    Open,
    Close(Option<Error>),
    CreateSession{
        tx: Sender<SessionFrame>,
        responder: oneshot::Sender<(u16, usize)>
    },
    DropSession(usize)
}

pub enum SessionControl {
    Begin,
    End,
}

pub enum LinkControl {
    
}