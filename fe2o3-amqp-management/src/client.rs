use fe2o3_amqp::{
    link::{DetachError, SendError},
    session::SessionHandle,
    Delivery, Receiver, Sender,
};
use fe2o3_amqp_types::{
    messaging::{ApplicationProperties, MessageId, Outcome, Properties},
    primitives::SimpleValue,
};

use crate::{
    error::{AttachError, Error, StatusError},
    operations::{
        CreateRequest, CreateResponse, DeleteRequest, DeleteResponse, DeregisterRequest,
        DeregisterResponse, GetAnnotationsRequest, GetAnnotationsResponse, GetAttributesRequest,
        GetAttributesResponse, GetMgmtNodesRequest, GetMgmtNodesResponse, GetOperationsRequest,
        GetOperationsResponse, GetTypesRequest, GetTypesResponse, QueryRequest, QueryResponse,
        ReadRequest, ReadResponse, RegisterRequest, RegisterResponse, UpdateRequest,
        UpdateResponse,
    },
    request::MessageSerializer,
    response::{MessageDeserializer, Response, ResponseMessageProperties},
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

    operation!(create, 'a,  CreateRequest<'a>, CreateResponse);

    operation!(read, 'a, ReadRequest<'a>, ReadResponse);

    operation!(update, 'a, UpdateRequest<'a>, UpdateResponse);

    operation!(delete, 'a, DeleteRequest<'a>, DeleteResponse);

    operation!(query, 'a, QueryRequest<'a>, QueryResponse);

    operation!(get_types, 'a, GetTypesRequest<'a>, GetTypesResponse);

    operation!(get_annotations, 'a, GetAnnotationsRequest<'a>, GetAnnotationsResponse);

    operation!(get_attributes, 'a, GetAttributesRequest<'a>, GetAttributesResponse);

    operation!(get_operations, 'a, GetOperationsRequest<'a>, GetOperationsResponse);

    operation!(get_mgmt_nodes, GetMgmtNodesRequest, GetMgmtNodesResponse);

    operation!(register, 'a, RegisterRequest<'a>, RegisterResponse);

    operation!(deregister, 'a, DeregisterRequest<'a>, DeregisterResponse);

    pub async fn send_request(
        &mut self,
        operation: impl MessageSerializer,
        entity_type: impl Into<String>,
        locales: impl Into<Option<String>>,
    ) -> Result<Outcome, SendError> {
        let mut message = operation.into_message();
        let application_properties = message
            .application_properties
            .get_or_insert(ApplicationProperties::default());
        application_properties.insert(
            String::from("type"),
            SimpleValue::String(entity_type.into()),
        );
        if let Some(locales) = locales.into() {
            application_properties
                .insert(String::from("locales"), SimpleValue::String(locales.into()));
        }

        let properties = message.properties.get_or_insert(Properties::default());
        properties.message_id = Some(MessageId::from(self.req_id));
        properties.reply_to = Some(self.client_node_addr.clone());

        self.req_id += 1;

        self.sender.send(message).await
    }

    pub async fn recv_response<O, T>(&mut self) -> Result<Response<O>, Error>
    where
        O: MessageDeserializer<T>,
        O::Error: Into<Error>,
        for<'de> T: serde::de::Deserialize<'de> + std::fmt::Debug + Send,
    {
        let delivery: Delivery<T> = self.receiver.recv().await?;
        self.receiver.accept(&delivery).await?;

        let mut message = delivery.into_message();
        let properties = ResponseMessageProperties::try_take_from_message(&mut message)?;
        let operation = MessageDeserializer::from_message(message).map_err(Into::into)?;

        Ok(Response::from_parts(properties, operation))
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
