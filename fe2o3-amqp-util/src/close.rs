use std::convert::Infallible;

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::{net::TcpStream, io::BufStream};

#[async_trait]
pub trait AsyncClose: Send + Sync + Unpin {
    type Message: Send;
    type Error;

    async fn close_with_message(&mut self, message: Option<Self::Message>) -> Result<(), Self::Error>;
    async fn close(&mut self) -> Result<(), Self::Error> {
        self.close_with_message(None).await
    }
}

#[async_trait]
impl<S> AsyncClose for BufStream<S> 
where
    S: AsyncRead + AsyncWrite + Send + Sync + Unpin,
{
    type Message = ();
    type Error = Infallible;

    async fn close_with_message(&mut self, _: Option<Self::Message>) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[cfg(feature = "tokio-net")]
#[async_trait]
impl AsyncClose for TcpStream {
    type Message = ();
    type Error = Infallible;

    async fn close_with_message(&mut self, _: Option<Self::Message>) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[cfg(feature = "tokio-rustls")]
#[async_trait]
impl<S> AsyncClose for tokio_rustls::TlsStream<S> 
where
    S: Send + Sync + Unpin,
{
    type Message = ();
    type Error = Infallible;

    async fn close_with_message(&mut self, _: Option<Self::Message>) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[cfg(feature = "tokio-rustls")]
#[async_trait]
impl<S> AsyncClose for tokio_rustls::client::TlsStream<S> 
where
    S: Send + Sync + Unpin,
{
    type Message = ();
    type Error = Infallible;

    async fn close_with_message(&mut self, _: Option<Self::Message>) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[cfg(feature = "tokio-rustls")]
#[async_trait]
impl<S> AsyncClose for tokio_rustls::server::TlsStream<S> 
where
    S: Send + Sync + Unpin,
{
    type Message = ();
    type Error = Infallible;

    async fn close_with_message(&mut self, _: Option<Self::Message>) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[cfg(feature = "tokio-native-tls")]
#[async_trait]
impl<S> AsyncClose for tokio_native_tls::TlsStream<S> 
where
    S: Send + Sync + Unpin,
{
    type Message = ();
    type Error = Infallible;

    async fn close_with_message(&mut self, _: Option<Self::Message>) -> Result<(), Self::Error> {
        Ok(())
    }
}
