//! Supported SASL mechanisms

use fe2o3_amqp_types::{
    primitives::Symbol,
    sasl::{SaslChallenge, SaslCode, SaslInit, SaslOutcome, SaslResponse},
};

use crate::sasl_profile::PLAIN;

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
    fn mechanisms(&self) -> Vec<Symbol>;

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

/// A naive acceptor for SASL PLAIN mechanism
#[derive(Debug)]
pub struct SaslPlainMechanism {
    username: String,
    password: String,
}

impl SaslPlainMechanism {
    fn validate_init(&self, init: SaslInit) -> Option<SaslCode> {
        let response = init.initial_response?.into_vec();

        let mut split = response.split(|b| *b == 0u8);
        let _authzid = split.next()?;
        let authcid = split.next()?;
        let passwd = split.next()?;
        Some(self.validate_credential(authcid, passwd))
    }

    fn validate_credential(&self, authcid: &[u8], passwd: &[u8]) -> SaslCode {
        if self.username.as_bytes() == authcid && self.password.as_bytes() == passwd {
            SaslCode::Ok
        } else {
            SaslCode::Auth
        }
    }
}

impl SaslAcceptor for SaslPlainMechanism {
    fn mechanisms(&self) -> Vec<Symbol> {
        vec![Symbol::from(PLAIN)]
    }

    fn on_init(&self, init: SaslInit) -> SaslServerFrame {
        let code = self.validate_init(init).unwrap_or(SaslCode::Auth);
        let outcome = SaslOutcome {
            code,
            additional_data: None,
        };
        SaslServerFrame::Outcome(outcome)
    }

    fn on_response(&self, _response: SaslResponse) -> SaslServerFrame {
        // This is not expected
        let outcome = SaslOutcome {
            code: SaslCode::Sys,
            additional_data: None,
        };
        SaslServerFrame::Outcome(outcome)
    }
}