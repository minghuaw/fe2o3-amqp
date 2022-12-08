//! Implements the CBS client

use std::borrow::Cow;

use fe2o3_amqp::{link::DetachError, session::SessionHandle};
use fe2o3_amqp_management::{
    client::MgmtClient,
    error::{AttachError, Error as MgmtError},
};

use crate::{
    constants::{CBS_NODE_ADDR, DEFAULT_CBS_CLIENT_NODE},
    put_token::{PutTokenRequest, PutTokenResponse},
    token::CbsToken,
};

/// CBS client
///
/// The connection should be opened with an ANONYMOUS SASL profile.
#[derive(Debug)]
pub struct CbsClient {
    mgmt_client: MgmtClient,
}

impl CbsClient {
    /// Attach a new CBS client to the session
    pub async fn attach(session: &mut SessionHandle<()>) -> Result<Self, AttachError> {
        let mgmt_client = MgmtClient::builder()
            .management_node_address(CBS_NODE_ADDR)
            .client_node_addr(DEFAULT_CBS_CLIENT_NODE)
            .attach(session)
            .await?;

        Ok(Self { mgmt_client })
    }

    /// Close the CBS client
    pub async fn close(self) -> Result<(), DetachError> {
        self.mgmt_client.close().await
    }

    /// Put a CBS token
    pub async fn put_token<'a>(
        &mut self,
        name: impl Into<Cow<'a, str>>,
        token: CbsToken<'a>,
    ) -> Result<(), MgmtError> {
        let entity_type = token.token_type;
        let req = PutTokenRequest::new(
            name,
            token.token_value,
            token.expires_at_utc,
            entity_type,
            None,
        );
        let _res: PutTokenResponse = self.mgmt_client.call(req).await?;
        Ok(())
    }
}
