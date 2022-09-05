//!

/// Determins how TLS connection will be established implicitly
///
/// For more details, please see [part 5.2 of the core
/// spec](http://docs.oasis-open.org/amqp/core/v1.0/os/amqp-core-security-v1.0-os.html#section-tls).
#[derive(Debug, Clone)]
pub enum TlsEstablishment {
    /// To establish a TLS session, each peer MUST start by sending a protocol header before commencing with TLS negotiation.
    ExchangeHeader,

    /// 5.2.1 As an alternative, implementations MAY run a pure TLS server, i.e., one that does not expect the initial TLS invoking handshake
    Alternative,
}

impl Default for TlsEstablishment {
    fn default() -> Self {
        Self::ExchangeHeader
    }
}
