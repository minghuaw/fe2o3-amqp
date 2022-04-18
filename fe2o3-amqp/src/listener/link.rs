//! Link Listener

// /// Listener for incoming link
// #[derive(Debug)]
// pub struct LinkListener {}

use fe2o3_amqp_types::definitions::{SenderSettleMode, ReceiverSettleMode};

/// An acceptor for incoming links
#[derive(Debug)]
pub struct LinkAcceptor {
    /// Supported sender settle modes
    pub(crate) snd_settle_modes: Vec<SenderSettleMode>,
    /// Supported receiver settle modes
    pub(crate) rcv_settle_modes: Vec<ReceiverSettleMode>,
}

/// A link on the listener side
#[derive(Debug)]
pub struct ListenerLink {

}
