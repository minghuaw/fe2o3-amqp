//! Experimental implementation of AMQP 1.0 CBS extension protocol

use std::future::Future;

use token::CbsToken;

pub mod client;
pub mod constants;
pub mod put_token;
pub mod token;

pub trait CbsTokenProvider {
    type Error;

    fn get_token(
        &mut self,
        container_id: impl AsRef<str>,
        resource_id: impl AsRef<str>,
        claims: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<CbsToken, Self::Error>;
}

/// TODO: This will be updated when GAT is stablized
pub trait AsyncCbsTokenProvider {
    type Error;
    type Fut<'a>: Future<Output = Result<CbsToken<'a>, Self::Error>>
    where
        Self: 'a;

    fn get_token_async(
        &mut self,
        container_id: impl AsRef<str>,
        resource_id: impl AsRef<str>,
        claims: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Self::Fut<'_>;
}
