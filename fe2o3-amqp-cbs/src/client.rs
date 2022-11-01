use fe2o3_amqp::{link::DetachError, session::SessionHandle};
use fe2o3_amqp_management::{
    client::MgmtClient,
    error::{AttachError, Error as MgmtError, StatusError},
    response::Response,
};

use crate::{
    constants::{CBS_NODE_ADDR, DEFAULT_CBS_CLIENT_NODE},
    put_token::{PutTokenRequest, PutTokenResponse}, token::CbsToken,
};

/// CBS client
///
/// The connection should be opened with an ANONYMOUS SASL profile.
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
        token: CbsToken<'_>,
    ) -> Result<(), MgmtError> {
        let entity_type = token.token_type.clone();
        let req = PutTokenRequest::from(token);
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
