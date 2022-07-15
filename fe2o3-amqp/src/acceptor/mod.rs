//! Acceptors for fine control over incoming connections, sessions, and links

pub mod builder;
pub mod connection;
pub mod error;
pub mod link;
pub mod local_receiver_link;
pub mod local_sender_link;
pub mod sasl_acceptor;
pub mod session;

use fe2o3_amqp_types::{
    definitions::{ReceiverSettleMode, SenderSettleMode},
    performatives::Begin,
};

pub use self::connection::{ConnectionAcceptor, ListenerConnectionHandle};
pub use self::link::{LinkAcceptor, LinkEndpoint};
pub use self::sasl_acceptor::{SaslAcceptor, SaslAnonymousMechanism, SaslPlainMechanism};
pub use self::session::{ListenerSessionHandle, SessionAcceptor};

/// A half established session that is initiated by the remote peer
#[derive(Debug)]
pub struct IncomingSession {
    /// The (remote) channel of incoming session
    pub channel: u16,

    /// The Begin performative sent by the remote peer
    pub begin: Begin,
}

/// The supported sender-settle-modes for the link acceptor
#[derive(Debug, Clone)]
pub enum SupportedSenderSettleModes {
    /// Only supports `SenderSettleMode::Unsettled`
    Unsettled,
    /// Only supports `SenderSettleMode::Settled`
    Settled,
    /// Only supports `SenderSettleMode::Mixed`
    Mixed,
    /// Supports `SenderSettleMode::Unsettled` and `SenderSettleMode::Settled`
    UnsettledAndSettled,
    /// Supports `SenderSettleMode::Unsettled` and `SenderSettleMode::Mixed`
    UnsettledAndMixed,
    /// Supports `SenderSettleMode::Settled` and `SenderSettleMode::Mixed`
    SettledAndMixed,
    /// Supports all three variants of `SenderSettleMode`
    All,
}

/// Defaults to `All`
impl Default for SupportedSenderSettleModes {
    fn default() -> Self {
        Self::All
    }
}

impl SupportedSenderSettleModes {
    /// Determines whether a mode is supported or not
    pub fn supports(&self, mode: &SenderSettleMode) -> bool {
        match mode {
            SenderSettleMode::Unsettled => match self {
                SupportedSenderSettleModes::Unsettled => true,
                SupportedSenderSettleModes::Settled => false,
                SupportedSenderSettleModes::Mixed => false,
                SupportedSenderSettleModes::UnsettledAndSettled => true,
                SupportedSenderSettleModes::UnsettledAndMixed => true,
                SupportedSenderSettleModes::SettledAndMixed => false,
                SupportedSenderSettleModes::All => true,
            },
            SenderSettleMode::Settled => match self {
                SupportedSenderSettleModes::Unsettled => false,
                SupportedSenderSettleModes::Settled => true,
                SupportedSenderSettleModes::Mixed => false,
                SupportedSenderSettleModes::UnsettledAndSettled => true,
                SupportedSenderSettleModes::UnsettledAndMixed => false,
                SupportedSenderSettleModes::SettledAndMixed => true,
                SupportedSenderSettleModes::All => true,
            },
            SenderSettleMode::Mixed => match self {
                SupportedSenderSettleModes::Unsettled => false,
                SupportedSenderSettleModes::Settled => false,
                SupportedSenderSettleModes::Mixed => true,
                SupportedSenderSettleModes::UnsettledAndSettled => false,
                SupportedSenderSettleModes::UnsettledAndMixed => true,
                SupportedSenderSettleModes::SettledAndMixed => true,
                SupportedSenderSettleModes::All => true,
            },
        }
    }
}

/// The supported receiver-settle-modes for the link acceptor
#[derive(Debug, Clone)]
pub enum SupportedReceiverSettleModes {
    /// Only supports `ReceiverSettleMode::First`
    First,
    /// Only supports `ReceiverSettleMode::Second`
    Second,
    /// Supports both variants of `ReceiverSettleMode`
    Both,
}

/// Defaults to `Both`
impl Default for SupportedReceiverSettleModes {
    fn default() -> Self {
        Self::Both
    }
}

impl SupportedReceiverSettleModes {
    /// Determines whether a receiver settle mode is supported or not
    pub fn supports(&self, mode: &ReceiverSettleMode) -> bool {
        match mode {
            ReceiverSettleMode::First => match self {
                SupportedReceiverSettleModes::First => true,
                SupportedReceiverSettleModes::Second => false,
                SupportedReceiverSettleModes::Both => true,
            },
            ReceiverSettleMode::Second => match self {
                SupportedReceiverSettleModes::First => false,
                SupportedReceiverSettleModes::Second => true,
                SupportedReceiverSettleModes::Both => true,
            },
        }
    }
}
