//! Listeners

pub mod builder;
pub mod connection;
pub mod link;
pub mod sasl_acceptor;
pub mod session;

use fe2o3_amqp_types::performatives::{Begin, Attach};

pub use self::connection::*;

/// A half established session that is initiated by the remote peer
#[derive(Debug)]
pub struct IncomingSession {
    channel: u16,
    begin: Begin,
}

impl IncomingSession {
    /// Get a reference to the received Begin performative
    pub fn begin(&self) -> &Begin {
        &self.begin
    }

    /// Gets the incoming channel for the incoming session
    pub fn channel(&self) -> u16 {
        self.channel
    }
}

// /// A half established link that is initiated by the remote peer
// #[derive(Debug)]
// pub struct IncomingLink {
//     attach: Attach,
// }

/// An alias that represents a half connected incoming link
pub type IncomingLink = Attach;

/// The supported sender-settle-modes for the link acceptor
#[derive(Debug)]
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

/// TODO: defaults to `All`
impl Default for SupportedSenderSettleModes {
    fn default() -> Self {
        Self::All
    }
}

/// The supported receiver-settle-modes for the link acceptor
#[derive(Debug)]
pub enum SupportedReceiverSettleModes {
    /// Only supports `ReceiverSettleMode::First`
    First,
    /// Only supports `ReceiverSettleMode::Second`
    Second,
    /// Supports both variants of `ReceiverSettleMode`
    Both,
}

/// TODO: defaults to `Both`
impl Default for SupportedReceiverSettleModes {
    fn default() -> Self {
        Self::Both
    }
}

// /// Trait for listeners
// #[async_trait]
// pub trait Listener {
//     /// Type of accepted IO stream
//     type Stream: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static;
//     // type Stream;

//     /// Type for local addr
//     type Addr;

//     /// Obtain the local address
//     fn local_addr(&self) -> Result<Self::Addr, io::Error>;

//     /// Accept an incoming stream
//     async fn accept(&self) -> Result<Self::Stream, io::Error>;
// }

// #[async_trait]
// impl Listener for TcpListener {
//     type Stream = TcpStream;
//     type Addr = SocketAddr;

//     fn local_addr(&self) -> Result<Self::Addr, io::Error> {
//         self.local_addr()
//     }

//     async fn accept(&self) -> Result<Self::Stream, io::Error> {
//         self.accept().await.map(|(socket, _)| socket)
//     }
// }
