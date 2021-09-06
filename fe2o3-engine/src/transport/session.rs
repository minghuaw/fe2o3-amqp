use tokio::sync::mpsc::Sender;

use super::amqp::Frame;

pub type SessionId = u16; // FIXME: each channel corresponds to a session, so number of sessions should not exceeds that of channel

pub struct SessionHandle {
    id: SessionId,
    tx: Sender<Frame>,
}