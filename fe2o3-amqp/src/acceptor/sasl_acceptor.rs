//! Supported SASL mechanisms

use std::sync::Arc;

use fe2o3_amqp_types::{
    primitives::{Array, Symbol},
    sasl::{SaslChallenge, SaslCode, SaslInit, SaslMechanisms, SaslOutcome, SaslResponse},
};

use crate::sasl_profile::{ANONYMOUS, PLAIN};

/// SASL frames sent by server, excluding the initial mechanism frame
#[derive(Debug)]
pub enum SaslServerFrame {
    /// SASL challenge frame
    Challenge(SaslChallenge),

    /// SASL outcome frame
    Outcome(SaslOutcome),
}

/// Server side SASL negotiation
pub trait SaslAcceptor: Clone {
    /// List of supported mechanisms
    fn mechanisms(&self) -> Array<Symbol>;

    /// Responde to a SaslInit frame
    fn on_init(&mut self, init: SaslInit) -> SaslServerFrame;

    /// Respond to a SaslResponse frame
    fn on_response(&mut self, response: SaslResponse) -> SaslServerFrame;
}

/// Extension trait of SaslAcceptor
pub trait SaslAcceptorExt: SaslAcceptor {
    /// Collects the supported sasl-server-mechanisms into a SaslMechanism frame.
    ///
    /// A list of one element with its value as the SASL mechanism ANONYMOUS will be
    /// returned if there is ZERO supported mechanism.
    ///
    /// It is invalid for this list to be null or empty. If the sending peer does not require
    /// its partner to authenticate with it, then it SHOULD send a list of one element with
    /// its value as the SASL mechanism ANONYMOUS.
    fn sasl_mechanisms(&self) -> SaslMechanisms {
        let server_mechanisms = self.mechanisms();

        if server_mechanisms.0.is_empty() {
            SaslMechanisms::default()
        } else {
            SaslMechanisms {
                sasl_server_mechanisms: server_mechanisms,
            }
        }
    }
}

impl<T: SaslAcceptor> SaslAcceptorExt for T {}

// /// Supported SASL mechanism
// #[derive(Debug)]
// pub enum Mechanism {
//     /// SASL PLAIN mechanism
//     Plain,
// }

/// A naive acceptor for SASL PLAIN mechanism
#[derive(Debug, Clone)]
pub struct SaslPlainMechanism {
    username: Arc<String>,
    password: Arc<String>,
}

impl SaslPlainMechanism {
    /// Creates a new PLAIN mechanism acceptor
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: Arc::new(username.into()),
            password: Arc::new(password.into()),
        }
    }
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
    fn mechanisms(&self) -> Array<Symbol> {
        Array::from(vec![Symbol::from(PLAIN)])
    }

    fn on_init(&mut self, init: SaslInit) -> SaslServerFrame {
        let code = self.validate_init(init).unwrap_or(SaslCode::Auth);
        let outcome = SaslOutcome {
            code,
            additional_data: None,
        };
        SaslServerFrame::Outcome(outcome)
    }

    fn on_response(&mut self, _response: SaslResponse) -> SaslServerFrame {
        // This is not expected
        let outcome = SaslOutcome {
            code: SaslCode::Sys,
            additional_data: None,
        };
        SaslServerFrame::Outcome(outcome)
    }
}

/// A SASL Anonymous acceptor that is going to accept anything
#[derive(Debug, Clone)]
pub struct SaslAnonymousMechanism {}

impl SaslAnonymousMechanism {
    /// Creates a new SASL Anonymouse mechanism
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SaslAnonymousMechanism {
    fn default() -> Self {
        Self::new()
    }
}

impl SaslAcceptor for SaslAnonymousMechanism {
    fn mechanisms(&self) -> Array<Symbol> {
        Array::from(vec![Symbol::from(ANONYMOUS)])
    }

    fn on_init(&mut self, _init: SaslInit) -> SaslServerFrame {
        let code = SaslCode::Ok;
        let outcome = SaslOutcome {
            code,
            additional_data: None,
        };
        SaslServerFrame::Outcome(outcome)
    }

    fn on_response(&mut self, _response: SaslResponse) -> SaslServerFrame {
        let code = SaslCode::Ok;
        let outcome = SaslOutcome {
            code,
            additional_data: None,
        };
        SaslServerFrame::Outcome(outcome)
    }
}
