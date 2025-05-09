use std::io;

use http::Response;
use tungstenite::{
    error::{CapacityError, ProtocolError, TlsError, UrlError},
    Message,
};

pub type HttpResponse = Response<Option<Vec<u8>>>;

/// Error with websocket binding
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A `tungsteninte::Error::ConnectionClosed` error
    #[error("Connection closed normally")]
    ConnectionClosed,

    /// A `tungsteninte::Error::AlreadyClosed` error
    #[error("Trying to work with closed connection")]
    AlreadyClosed,

    /// A `tungsteninte::Error::Io` error
    #[error("IO error: {0}")]
    Io(io::Error),

    /// A `tungsteninte::Error::Tls` error
    #[error("TLS error: {0}")]
    Tls(TlsError),

    /// A `tungsteninte::Error::Capacity` error
    #[error("Space limit exceeded: {0}")]
    Capacity(CapacityError),

    /// A `tungsteninte::Error::Protocol` error
    #[error("WebSocket protocol error: {0}")]
    Protocol(ProtocolError),

    /// A `tungsteninte::Error::WriteBufferFull` error
    #[error("Send queue is full")]
    WriteBufferFull(Message),

    /// A `tungsteninte::Error::Utf8` error
    #[error("UTF-8 encoding error")]
    Utf8,

    /// A `tungsteninte::Error::Url` error
    #[error("URL error: {0}")]
    Url(UrlError),

    /// A `tungsteninte::Error::Http` error
    #[error("HTTP error: {}", .0.status())]
    Http(Box<HttpResponse>),

    /// A `tungsteninte::Error::HttpFormat` error
    #[error("HTTP format error: {0}")]
    HttpFormat(http::Error),

    /// The client expects a status code 101
    #[error("A status code 101 is expected")]
    StatucCodeIsNotSwitchingProtocols,

    /// The HTTP header key "Sec-WebSocket-Protocol" is not found
    #[error("No \"Sec-WebSocket-Protocol\" header")]
    MissingSecWebSocketProtocol,

    /// The client expects an HTTP Sec-WebSocket-Protocol equal to the US-ASCII text string “amqp”
    #[error("Expect \"Sec-WebSocket-Protocol\" equal to \"amqp\"")]
    SecWebSocketProtocolIsNotAmqp,

    /// `tungstenite::Error::AttackAttempt` error. Attack attempt detected.
    #[error("Attack attempt detected")]
    AttackAttempt,
}

impl<T: Into<tungstenite::Error>> From<T> for Error {
    fn from(value: T) -> Self {
        let value = value.into();
        match value {
            tungstenite::Error::ConnectionClosed => Self::ConnectionClosed,
            tungstenite::Error::AlreadyClosed => Self::AlreadyClosed,
            tungstenite::Error::Io(val) => Self::Io(val),
            tungstenite::Error::Tls(val) => Self::Tls(val),
            tungstenite::Error::Capacity(val) => Self::Capacity(val),
            tungstenite::Error::Protocol(val) => Self::Protocol(val),
            tungstenite::Error::WriteBufferFull(val) => Self::WriteBufferFull(val),
            tungstenite::Error::Utf8 => Self::Utf8,
            tungstenite::Error::Url(val) => Self::Url(val),
            tungstenite::Error::Http(val) => Self::Http(Box::new(val)),
            tungstenite::Error::HttpFormat(val) => Self::HttpFormat(val),
            tungstenite::Error::AttackAttempt => Self::AttackAttempt,
        }
    }
}
