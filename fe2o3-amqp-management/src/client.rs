use fe2o3_amqp::{
    link::{DetachError, SendError},
    session::SessionHandle,
    Delivery, Receiver, Sender,
};
use fe2o3_amqp_types::{
    messaging::{ApplicationProperties, FromBody, IntoBody, MessageId, Outcome, Properties},
    primitives::SimpleValue,
};

use crate::{
    error::{AttachError, Error, StatusError},
    // operations::{
    //     CreateRequest, CreateResponse, DeleteRequest, DeleteResponse, DeregisterRequest,
    //     DeregisterResponse, GetAnnotationsRequest, GetAnnotationsResponse, GetAttributesRequest,
    //     GetAttributesResponse, GetMgmtNodesRequest, GetMgmtNodesResponse, GetOperationsRequest,
    //     GetOperationsResponse, GetTypesRequest, GetTypesResponse, QueryRequest, QueryResponse,
    //     ReadRequest, ReadResponse, RegisterRequest, RegisterResponse, UpdateRequest,
    //     UpdateResponse,
    // },
    request::IntoMessage,
    response::{FromMessage},
    DEFAULT_CLIENT_NODE_ADDRESS, MANAGEMENT_NODE_ADDRESS,
};

pub struct MgmtClient {
    req_id: u64,
    client_node_addr: String,
    sender: Sender,
    receiver: Receiver,
}

macro_rules! operation {
    ($op:ident, $lt:lifetime, $req_ty:ty, $res_ty:ty) => {
        pub async fn $op<$lt>(
            &mut self,
            req: $req_ty,
            entity_type: impl Into<String>,
            locales: Option<String>,
        ) -> Result<$res_ty, Error> {
            self.send_request(req, entity_type, locales)
                .await?
                .accepted_or_else(|o| Error::NotAccepted(o))?;
            let response: Response<$res_ty> = self.recv_response().await?;
            match response.status_code.0.get() {
                <$res_ty>::STATUS_CODE => Ok(response.operation),
                _ => Err(StatusError {
                    code: response.status_code,
                    description: response.status_description,
                }
                .into()),
            }
        }
    };

    ($op:ident, $req_ty:ty, $res_ty:ty) => {
        pub async fn $op(
            &mut self,
            req: $req_ty,
            entity_type: impl Into<String>,
            locales: Option<String>,
        ) -> Result<$res_ty, Error> {
            self.send_request(req, entity_type, locales)
                .await?
                .accepted_or_else(|o| Error::NotAccepted(o))?;
            let response: Response<$res_ty> = self.recv_response().await?;
            match response.status_code.0.get() {
                <$res_ty>::STATUS_CODE => Ok(response.operation),
                _ => Err(StatusError {
                    code: response.status_code,
                    description: response.status_description,
                }
                .into()),
            }
        }
    };
}

impl MgmtClient {
    pub fn builder() -> MgmtClientBuilder {
        MgmtClientBuilder::default()
    }

    pub async fn attach<R>(
        session: &mut SessionHandle<R>,
        client_node_addr: impl Into<String>,
    ) -> Result<Self, AttachError> {
        Self::builder()
            .client_node_addr(client_node_addr)
            .attach(session)
            .await
    }

    pub async fn close(self) -> Result<(), DetachError> {
        self.sender.close().await?;
        self.receiver.close().await?;
        Ok(())
    }

    // operation!(create, 'a,  CreateRequest<'a>, CreateResponse);

    // operation!(read, 'a, ReadRequest<'a>, ReadResponse);

    // operation!(update, 'a, UpdateRequest<'a>, UpdateResponse);

    // operation!(delete, 'a, DeleteRequest<'a>, DeleteResponse);

    // operation!(query, 'a, QueryRequest<'a>, QueryResponse);

    // operation!(get_types, 'a, GetTypesRequest<'a>, GetTypesResponse);

    // operation!(get_annotations, 'a, GetAnnotationsRequest<'a>, GetAnnotationsResponse);

    // operation!(get_attributes, 'a, GetAttributesRequest<'a>, GetAttributesResponse);

    // operation!(get_operations, 'a, GetOperationsRequest<'a>, GetOperationsResponse);

    // operation!(get_mgmt_nodes, GetMgmtNodesRequest, GetMgmtNodesResponse);

    // operation!(register, 'a, RegisterRequest<'a>, RegisterResponse);

    // operation!(deregister, 'a, DeregisterRequest<'a>, DeregisterResponse);

    pub async fn send_request(
        &mut self,
        request: impl IntoMessage,
    ) -> Result<Outcome, SendError> {
        let mut message = request.into_message().map_body(IntoBody::into_body);

        // Only insert the request-id if it's not already set
        let properties = message.properties.get_or_insert(Properties::default());
        properties.message_id.get_or_insert({
            let message_id = MessageId::from(self.req_id);
            self.req_id += 1;
            message_id
        });
        properties.reply_to.get_or_insert(self.client_node_addr.clone());

        self.sender.send(message).await
    }
    pub async fn recv_response<Res>(&mut self) -> Result<Res, Error>
    where
        Res: FromMessage,
        Res::Error: Into<Error>,
        for<'de> Res::Body: FromBody<'de> + std::fmt::Debug + Send,
    {
        let delivery: Delivery<Res::Body> = self.receiver.recv().await?;
        self.receiver.accept(&delivery).await?;

        Res::from_message(delivery.into_message()).map_err(Into::into)
    }

    pub async fn call<Req, Res>(
        &mut self,
        request: Req,
    ) -> Result<Res, Error>
    where
        Req: IntoMessage,
        Res: FromMessage,
        Res::Error: Into<Error>,
        for<'de> Res::Body: FromBody<'de> + std::fmt::Debug + Send,
    {
        self.send_request(request).await?;
        self.recv_response().await
    }
}

pub struct MgmtClientBuilder {
    mgmt_node_addr: String,
    client_node_addr: String,
}

impl Default for MgmtClientBuilder {
    fn default() -> Self {
        MgmtClientBuilder {
            mgmt_node_addr: String::from(MANAGEMENT_NODE_ADDRESS),
            client_node_addr: String::from(DEFAULT_CLIENT_NODE_ADDRESS),
        }
    }
}

impl MgmtClientBuilder {
    pub fn management_node_address(mut self, mgmt_node_addr: impl Into<String>) -> Self {
        self.mgmt_node_addr = mgmt_node_addr.into();
        self
    }

    pub fn client_node_addr(mut self, client_node_addr: impl Into<String>) -> Self {
        self.client_node_addr = client_node_addr.into();
        self
    }

    pub async fn attach<R>(
        self,
        session: &mut SessionHandle<R>,
    ) -> Result<MgmtClient, AttachError> {
        let sender = Sender::builder()
            .name(format!("{}-mgmt-sender", self.client_node_addr))
            .target(&self.mgmt_node_addr)
            .attach(session)
            .await?;
        let receiver = Receiver::builder()
            .name(format!("{}-mgmt-receiver", self.client_node_addr))
            .source(self.mgmt_node_addr)
            .target(&self.client_node_addr)
            .attach(session)
            .await?;

        Ok(MgmtClient {
            req_id: 0,
            client_node_addr: self.client_node_addr,
            sender,
            receiver,
        })
    }
}
