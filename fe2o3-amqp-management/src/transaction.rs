//!  Transaction extension for AMQP management link

use async_trait::async_trait;
use fe2o3_amqp::{transaction::{TransactionExt, Transaction, PostError}};
use fe2o3_amqp_types::messaging::{Outcome, FromBody, IntoBody};

use crate::{MgmtClient, Request, error::{Error, TxnMgmtLinkError}, Response};

/// Transaction extension for AMQP management link
/// 
/// See https://github.com/Azure/azure-amqp/issues/237
#[async_trait]
pub trait ManagementLinkTransactionExt: TransactionExt {
    /// Send a request and wait for the outcome.
    async fn send_request<Req>(
        &self,
        client: &mut MgmtClient,
        request: Req
    ) -> Result<Outcome, PostError>
    where
        Req: Request + Send,
        Req::Body: Send,
        <<Req as Request>::Body as IntoBody>::Body: Send;

    /// Send a request and wait for the outcome.
    async fn call<Req, Res>(
        &self,
        client: &mut MgmtClient,
        request: Req,
    ) -> Result<Res, TxnMgmtLinkError>
    where
        Req: Request<Response = Res> + Send,
        Req::Body: Send,
        <<Req as Request>::Body as IntoBody>::Body: Send,
        Res: Response + Send,
        Res::Body: Send + Sync,
        Res::Error: Into<Error>,
        for<'de> Res::Body: FromBody<'de> + std::fmt::Debug + Send;
}

#[async_trait]
impl<'t> ManagementLinkTransactionExt for Transaction<'t> {
    async fn send_request<Req>(
        &self,
        client: &mut MgmtClient,
        request: Req
    ) -> Result<Outcome, PostError> 
    where
        Req: Request + Send,
        Req::Body: Send,
        <<Req as Request>::Body as IntoBody>::Body: Send,
    {
        let message = request.into_message().map_body(IntoBody::into_body);
        self.post(&mut client.sender, message).await
    }

    async fn call<Req, Res>(
        &self,
        client: &mut MgmtClient,
        request: Req,
    ) -> Result<Res, TxnMgmtLinkError>
    where
        Req: Request<Response = Res> + Send,
        Req::Body: Send,
        <<Req as Request>::Body as IntoBody>::Body: Send,
        Res: Response + Send,
        Res::Body: Send + Sync,
        Res::Error: Into<Error>,
        for<'de> Res::Body: FromBody<'de> + std::fmt::Debug + Send 
    {
        let outcome = self.send_request(client, request).await?;
        let _accepted = outcome.accepted_or_else(Error::NotAccepted)?;
        client.recv_response().await.map_err(Into::into)
    }
}