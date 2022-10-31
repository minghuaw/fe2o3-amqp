//! Experimental implementation of AMQP 1.0 CBS extension protocol

use std::{pin::Pin, future::Future};

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
        claims: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<CbsToken, Self::Error>;
}

pub trait AsyncCbsTokenProvider {
    type Error;

    fn get_token_async(
        &mut self,
        container_id: impl AsRef<str>,
        resource_id: impl AsRef<str>,
        claims: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Pin<Box<dyn Future<Output = Result<CbsToken, Self::Error>>>>, Self::Error>;   
}

// /// Open connection with `SaslProfile::Anonymous` and then perform CBS
// pub async fn open_connection_and_perform_cbs<'a, Mode, Tls>(
//     builder: connection::Builder<'a, Mode, Tls>,
// ) -> Result<ConnectionHandle<()>, ()> {
//     todo!()
// }
