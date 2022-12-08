//! Implements the CBS client

use std::borrow::Cow;

use fe2o3_amqp::{link::DetachError, session::SessionHandle, types::definitions::Fields};
use fe2o3_amqp_management::{
    client::{MgmtClient, MgmtClientBuilder},
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
    /// Create a new CBS client builder
    pub fn builder() -> CbsClientBuilder {
        CbsClientBuilder::default()
    }

    /// Attach a new CBS client to the session
    pub async fn attach(session: &mut SessionHandle<()>) -> Result<Self, AttachError> {
        Self::builder().attach(session).await
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

/// Builder for a CBS client
#[derive(Debug)]
pub struct CbsClientBuilder {
    inner: MgmtClientBuilder,
}

impl Default for CbsClientBuilder {
    fn default() -> Self {
        let inner = MgmtClient::builder()
            .management_node_address(CBS_NODE_ADDR)
            .client_node_addr(DEFAULT_CBS_CLIENT_NODE);
        Self { inner }
    }
}

impl CbsClientBuilder {
    /// Set the sender link properties.
    pub fn sender_properties(mut self, properties: Fields) -> Self {
        self.inner = self.inner.sender_properties(properties);
        self
    }

    /// Set the receiver link properties.
    pub fn receiver_properties(mut self, properties: Fields) -> Self {
        self.inner = self.inner.receiver_properties(properties);
        self
    }

    /// Set the management node address.
    pub fn management_node_address(mut self, mgmt_node_addr: impl Into<String>) -> Self {
        self.inner = self.inner.management_node_address(mgmt_node_addr);
        self
    }

    /// Set the client node address.
    pub fn client_node_addr(mut self, client_node_addr: impl Into<String>) -> Self {
        self.inner = self.inner.client_node_addr(client_node_addr);
        self
    }

    /// Attach a management client to a session.
    pub async fn attach<R>(self, session: &mut SessionHandle<R>) -> Result<CbsClient, AttachError> {
        let mgmt_client = self.inner.attach(session).await?;
        Ok(CbsClient { mgmt_client })
    }
}
