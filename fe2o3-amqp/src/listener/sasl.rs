//! Supported SASL mechanisms

use fe2o3_amqp_types::sasl::{SaslChallenge, SaslOutcome, SaslInit, SaslResponse};

use crate::frames::sasl;

/// SASL frames sent by server, excluding the initial mechanism frame
#[derive(Debug)]
pub enum SaslServerFrame {
    /// SASL challenge frame
    Challenge(SaslChallenge),

    /// SASL outcome frame
    Outcome(SaslOutcome),
}

/// Server side SASL negotiation
pub trait SaslAcceptor {
    /// List of supported mechanisms
    fn mechanisms(&self) -> Vec<String>;

    /// Responde to a SaslInit frame
    fn on_init(&self, init: SaslInit) -> SaslServerFrame;

    /// Respond to a SaslResponse frame
    fn on_response(&self, response: SaslResponse) -> SaslServerFrame;
}

/// Supported SASL mechanism
#[derive(Debug)]
pub enum Mechanism {
    /// SASL PLAIN mechanism
    Plain,
}

