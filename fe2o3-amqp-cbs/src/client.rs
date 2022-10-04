use std::borrow::Cow;

use fe2o3_amqp_management::{
    client::MgmtClient,
    error::{Error as MgmtError, StatusError},
    response::Response,
};

use crate::{
    error::CbsClientError,
    put_token::{PutTokenRequest, PutTokenResponse},
};

/// CBS client
pub struct CbsClient {
    mgmt_client: MgmtClient,
}

impl CbsClient {
    pub async fn attach() -> Result<Self, CbsClientError> {
        todo!()
    }

    pub async fn close(self) {
        todo!()
    }

    pub async fn put_token(
        &mut self,
        name: impl Into<Cow<'_, str>>,
        token: impl Into<Cow<'_, str>>,
        entity_type: impl Into<Cow<'_, str>>,
    ) -> Result<(), MgmtError> {
        let req = PutTokenRequest { name: name.into(), token: token.into() };
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
