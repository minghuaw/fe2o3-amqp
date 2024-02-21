#![deny(missing_docs, missing_debug_implementations)]

//! Experimental implementation of AMQP 1.0 CBS extension protocol
//!
//! Please note that because the CBS protocol is still in draft, this crate is expected to see
//! breaking changes in all future releases until the draft becomes stable.

use token::CbsToken;

pub mod client;
pub mod constants;
pub mod put_token;
pub mod token;

/// A trait for providing CBS tokens
pub trait CbsTokenProvider {
    /// The associated error type
    type Error;

    /// Get a CBS token
    fn get_token(
        &mut self,
        container_id: impl AsRef<str>,
        resource_id: impl AsRef<str>,
        claims: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<CbsToken, Self::Error>;
}

/// An async version of `CbsTokenProvider`
#[trait_variant::make(AsyncCbsTokenProvider: Send)]
pub trait LocalAsyncCbsTokenProvider {
    /// The associated error type
    type Error;

    /// Get a CBS token
    async fn get_token_async(
        &mut self,
        container_id: impl AsRef<str>,
        resource_id: impl AsRef<str>,
        claims: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<CbsToken<'_>, Self::Error>;
}
