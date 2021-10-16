use async_trait::async_trait;

/// Trait definition for endpoints (Connection, Session, Link)

#[async_trait]
pub trait Connection {
    type Error;
    // TODO: what should be in here?

    // async fn create_session(&mut self) -> Result<super::session::Session, EngineError>;
    async fn close(&mut self) -> Result<(), Self::Error>;
}

#[async_trait]
pub trait Session {
    type Error;

    async fn end(&mut self) -> Result<(), Self::Error>;
}

#[async_trait]
pub trait Link {
    type Error;

    async fn detach(&mut self) -> Result<(), Self::Error>;
}