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

    /// Reacting to remote Open frame
    async fn handle_remote_open(&mut self, channel: u16, open: &mut Open) -> Result<(), Self::Error>;

    /// Reacting to remote Begin frame
    async fn intercept_remote_begin(&mut self, channel: u16, begin: &mut Begin) -> Result<(), Self::Error>;

    /// Reacting to remote End frame
    async fn intercept_remote_end(&mut self, channel: u16, end: &mut End) -> Result<(), Self::Error>;

    /// Reacting to remote Close frame
    async fn handle_remote_close(&mut self, channel: u16, close: &mut Close) -> Result<(), Self::Error>;

    async fn handle_local_open(&mut self, channel: u16, open: &mut Open) -> Result<Frame, Self::Error>;

    async fn intercept_local_begin(&mut self, channel: u16, begin: &mut Begin) -> Result<Frame, Self::Error>;

    async fn intercept_local_end(&mut self, channel: u16, end: &mut End) -> Result<Frame, Self::Error>;

    async fn handle_local_close(&mut self, channel: u16, close: &mut Close) -> Result<Frame, Self::Error>;
}

#[async_trait]
pub trait Session {
    type Error;

    async fn handle_remote_begin() -> Result<(), Self::Error>;
    async fn intercept_remote_attach() -> Result<(), Self::Error>;
    async fn intercept_remote_flow() -> Result<(), Self::Error>;
    async fn intercept_remote_transfer() -> Result<(), Self::Error>;
    async fn intercept_remote_disposition() -> Result<(), Self::Error>;
    async fn intercept_remote_detach() -> Result<(), Self::Error>;
    async fn handle_remote_end() -> Result<(), Self::Error>;

    async fn handle_local_begin() -> Result<(), Self::Error>;
    async fn intercept_local_attach() -> Result<(), Self::Error>;
    async fn intercept_local_flow() -> Result<(), Self::Error>;
    async fn intercept_local_transfer() -> Result<(), Self::Error>;
    async fn intercept_local_disposition() -> Result<(), Self::Error>;
    async fn intercept_local_detach() -> Result<(), Self::Error>;
    async fn handle_local_end() -> Result<(), Self::Error>;
}

#[async_trait]
pub trait Link {
    type Error;

    async fn handle_remote_attach() -> Result<(), Self::Error>;
    async fn handle_remote_flow() -> Result<(), Self::Error>;
    async fn handle_remote_transfer() -> Result<(), Self::Error>;
    async fn handle_remote_disposition() -> Result<(), Self::Error>;
    async fn handle_remote_detach() -> Result<(), Self::Error>;

    async fn handle_local_attach() -> Result<(), Self::Error>;
    async fn handle_local_flow() -> Result<(), Self::Error>;
    async fn handle_local_transfer() -> Result<(), Self::Error>;
    async fn handle_local_disposition() -> Result<(), Self::Error>;
    async fn handle_local_detach() -> Result<(), Self::Error>;
}
