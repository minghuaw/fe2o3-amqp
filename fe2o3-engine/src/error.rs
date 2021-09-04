use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("IO Error: {0:?}")]
    Io(#[from] std::io::Error),

    #[error("Parse Error: {0}")]
    ParseError(#[from] fe2o3_amqp::Error),

    #[error("Unexpected Protocol ID {0:?}")]
    UnexpectedProtocolId(u8),

    #[error("Unexpected Protocol Header. Found {0:?}")]
    UnexpectedProtocolHeader([u8; 8]),
}
