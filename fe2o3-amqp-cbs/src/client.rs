use std::borrow::Cow;

use fe2o3_amqp::{link::DetachError, session::SessionHandle};
use fe2o3_amqp_management::{
    client::MgmtClient,
    error::{AttachError, Error as MgmtError, StatusError},
    response::Response,
};

use crate::{
    constants::{CBS_NODE_ADDR, DEFAULT_CBS_CLIENT_NODE},
    error::CbsClientError,
    put_token::{PutTokenRequest, PutTokenResponse},
};

/// CBS client
pub struct CbsClient {
    mgmt_client: MgmtClient,
}

impl CbsClient {
    pub async fn attach(session: &mut SessionHandle<()>) -> Result<Self, AttachError> {
        let mgmt_client = MgmtClient::builder()
            .management_node_address(CBS_NODE_ADDR)
            .client_node_addr(DEFAULT_CBS_CLIENT_NODE)
            .attach(session)
            .await?;

        Ok(Self { mgmt_client })
    }

    pub async fn close(self) -> Result<(), DetachError> {
        self.mgmt_client.close().await
    }

    pub async fn put_token(
        &mut self,
        name: impl Into<Cow<'_, str>>,
        token: impl Into<Cow<'_, str>>,
        entity_type: impl Into<Cow<'_, str>>,
    ) -> Result<(), MgmtError> {
        let req = PutTokenRequest {
            name: name.into(),
            token: token.into(),
        };
        let entity_type = entity_type.into();
        let _accepted = self
            .mgmt_client
            .send_request(req, entity_type, None)
            .await?
            .accepted_or_else(|o| MgmtError::NotAccepted(o))?;
        let response: Response<PutTokenResponse> = self.mgmt_client.recv_response().await?;

        match response.status_code.0.get() {
            PutTokenResponse::STATUS_CODE => Ok(()),
            _ => Err(StatusError {
                code: response.status_code,
                description: response.status_description,
            }
            .into()),
        }
    }
}
