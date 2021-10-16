//! Trait abstraction of Connection, Session and Link
//! 
//! Frame          Connection  Session  Link
//! ========================================
//! open               H
//! begin              I          H
//! attach                        I       H 
//! flow                          I       H 
//! transfer                      I       H 
//! disposition                   I       H 
//! detach                        I       H 
//! end                I          H
//! close              H
//! ----------------------------------------
//! Key:
//!     H: handled by the endpoint
//!     I: intercepted (endpoint examines
//!         the frame, but delegates
//!         further processing to another
//!         endpoint)

use async_trait::async_trait;
use fe2o3_amqp_types::performatives::{Begin, Close, End, Open};

use crate::transport::amqp::Frame;

#[async_trait]
pub trait Connection {
    type Error;
    type Session: Session;

    /// Reacting to remote Open frame
    async fn on_incoming_open(&mut self, channel: u16, open: &mut Open) -> Result<(), Self::Error>;

    /// Reacting to remote Begin frame
    async fn on_incoming_begin(&mut self, channel: u16, begin: &mut Begin) -> Result<(), Self::Error>;

    /// Reacting to remote End frame
    async fn on_incoming_end(&mut self, channel: u16, end: &mut End) -> Result<(), Self::Error>;

    /// Reacting to remote Close frame
    async fn on_incoming_close(&mut self, channel: u16, close: &mut Close) -> Result<(), Self::Error>;

    async fn on_outgoing_open(&mut self, channel: u16, open: &mut Open) -> Result<Frame, Self::Error>;

    async fn on_outgoing_begin(&mut self, channel: u16, begin: &mut Begin) -> Result<Frame, Self::Error>;

    async fn on_outgoing_end(&mut self, channel: u16, end: &mut End) -> Result<Frame, Self::Error>;

    async fn on_outgoing_close(&mut self, channel: u16, close: &mut Close) -> Result<Frame, Self::Error>;

    fn session_mut_by_incoming_channel(&mut self, channel: u16) -> &mut Self::Session;

    fn session_mut_by_outgoing_channel(&mut self, channel: u16) -> &mut Self::Session;
}

#[async_trait]
pub trait Session {
    type Error;

    async fn on_incoming_begin() -> Result<(), Self::Error>;
    async fn on_incoming_attach() -> Result<(), Self::Error>;
    async fn on_incoming_flow() -> Result<(), Self::Error>;
    async fn on_incoming_transfer() -> Result<(), Self::Error>;
    async fn on_incoming_disposition() -> Result<(), Self::Error>;
    async fn on_incoming_detach() -> Result<(), Self::Error>;
    async fn on_incoming_end() -> Result<(), Self::Error>;

    async fn on_outgoing_begin() -> Result<(), Self::Error>;
    async fn on_outgoing_attach() -> Result<(), Self::Error>;
    async fn on_outgoing_flow() -> Result<(), Self::Error>;
    async fn on_outgoing_transfer() -> Result<(), Self::Error>;
    async fn on_outgoing_disposition() -> Result<(), Self::Error>;
    async fn on_outgoing_detach() -> Result<(), Self::Error>;
    async fn on_outgoing_end() -> Result<(), Self::Error>;
}

#[async_trait]
pub trait Link {
    type Error;

    async fn on_incoming_attach() -> Result<(), Self::Error>;
    async fn on_incoming_flow() -> Result<(), Self::Error>;
    async fn on_incoming_transfer() -> Result<(), Self::Error>;
    async fn on_incoming_disposition() -> Result<(), Self::Error>;
    async fn on_incoming_detach() -> Result<(), Self::Error>;

    async fn on_outgoing_attach() -> Result<(), Self::Error>;
    async fn on_outgoing_flow() -> Result<(), Self::Error>;
    async fn on_outgoing_transfer() -> Result<(), Self::Error>;
    async fn on_outgoing_disposition() -> Result<(), Self::Error>;
    async fn on_outgoing_detach() -> Result<(), Self::Error>;
}
