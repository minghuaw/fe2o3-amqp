/// Error with websocket binding
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error from the underlying tungstenite crate.
    #[error(transparent)]
    Tungstenite(#[from] tungstenite::Error),

    /// The client expects a status code 101
    #[error("A status code 101 is expected")]
    StatucCodeIsNotSwitchingProtocols,

    /// The HTTP header key "Sec-WebSocket-Protocol" is not found
    #[error("No \"Sec-WebSocket-Protocol\" header")]
    MissingSecWebSocketProtocol,

    /// The client expects an HTTP Sec-WebSocket-Protocol equal to the US-ASCII text string “amqp”
    #[error("Expect \"Sec-WebSocket-Protocol\" equal to \"amqp\"")]
    SecWebSocketProtocolIsNotAmqp,
}
