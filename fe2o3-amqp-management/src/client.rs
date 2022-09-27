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
    error::{AttachError, Error},
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

    // pub async fn create(&mut self, req: CreateRequest, entity_type: impl Into<String>, locales: Option<String>) -> Result<CreateResponse, Error> {
    //     todo!()
    // }

    // pub async fn read(&mut self, req: ReadRequest, entity_type: impl Into<String>, locales: Option<String>) -> Result<ReadResponse, Error> {
    //     todo!()
    // }

    // pub async fn update(&mut self, req: UpdateRequest, entity_type: impl Into<String>, locales: Option<String>) -> Result<UpdateResponse, Error> {
    //     todo!()
    // }

    // pub async fn delete(&mut self, req: DeleteRequest, entity_type: impl Into<String>, locales: Option<String>) -> Result<DeleteResponse, Error> {
    //     todo!()
    // }

    // pub async fn query(&mut self, req: QueryRequest, entity_type: impl Into<String>, locales: Option<String>) -> Result<QueryResponse, Error> {
    //     todo!()
    // }

    // pub async fn get_types(&mut self, req: GetTypesRequest, entity_type: impl Into<String>, locales: Option<String>) -> Result<GetTypesResponse, Error> {
    //     todo!()
    // }

    // pub async fn get_annotations(&mut self, req: UpdateRequest, entity_type: impl Into<String>, locales: Option<String>) -> Result<UpdateResponse, Error> {
    //     todo!()
    // }

    // pub async fn delete(&mut self, req: DeleteRequest, entity_type: impl Into<String>, locales: Option<String>) -> Result<DeleteResponse, Error> {
    //     todo!()
    // }

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
