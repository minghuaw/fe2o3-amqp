//! Negotiation priority

/// Priority in protocol negotiation
#[derive(Debug, Clone)]
pub enum NegotiationPriority {
    /// Performs negotiation in the order of 
    /// 
    /// 1. TLS
    /// 2. SASL
    /// 3. AMQP
    TlsSaslAmqp,

    /// Performs negotiation in the order of 
    /// 
    /// 1. SASL
    /// 2. TLS
    /// 3. AMQP
    SaslTlsAmqp,
}

impl Default for NegotiationPriority {
    fn default() -> Self {
        Self::TlsSaslAmqp
    }
}