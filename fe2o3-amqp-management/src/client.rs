use fe2o3_amqp::{
    link::{DetachError, RecvError, SendError},
    session::SessionHandle,
    Receiver, Sender,
};
use fe2o3_amqp_types::messaging::{ApplicationProperties, MessageId, Outcome, Properties};

use crate::{
    error::{AttachError, Error},
    operations::OperationRequest,
    request::{MessageSerializer, Request},
    DEFAULT_CLIENT_NODE_ADDRESS, MANAGEMENT_NODE_ADDRESS,
};

pub struct MgmtClient {
    req_id: u64,
    client_node_addr: String,
    sender: Sender,
    receiver: Receiver,
}

macro_rules! send_operation_request {
    ($client:ident, $operation:ident, $req:ident) => {
        {
            let mut message = $operation.into_message();
            let application_properties = message
                .application_properties
                .get_or_insert(ApplicationProperties::default());
            application_properties.insert(String::from("type"), $req.mgmt_entity_type.into());
            if let Some(locales) = $req.locales {
                application_properties.insert(String::from("locales"), locales.into());
            }
    
            let properties = message.properties.get_or_insert(Properties::default());
            properties.message_id = Some(MessageId::from($client.req_id));
            properties.reply_to = Some($client.client_node_addr.clone());
    
            $client.sender.send(message).await
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

    async fn send_request(&mut self, req: Request) -> Result<Outcome, SendError> {
        match req.operation {
            OperationRequest::Create(op) => {
                send_operation_request!(self, op, req)
            }
            OperationRequest::Read(op) => {
                send_operation_request!(self, op, req)
            }
            OperationRequest::Update(op) => {
                send_operation_request!(self, op, req)
            }
            OperationRequest::Delete(op) => {
                send_operation_request!(self, op, req)
            }
            OperationRequest::Query(op) => {
                send_operation_request!(self, op, req)
            }
            OperationRequest::GetTypes(op) => {
                send_operation_request!(self, op, req)
            }
            OperationRequest::GetAnnotations(op) => {
                send_operation_request!(self, op, req)
            }
            OperationRequest::GetAttributes(op) => {
                send_operation_request!(self, op, req)
            }
            OperationRequest::GetOperations(op) => {
                send_operation_request!(self, op, req)
            }
            OperationRequest::GetMgmtNodes(op) => {
                send_operation_request!(self, op, req)
            }
            OperationRequest::Register(op) => {
                send_operation_request!(self, op, req)
            }
            OperationRequest::Deregister(op) => {
                send_operation_request!(self, op, req)
            }
        }
    }

    async fn recv_response(&mut self) -> Result<(), RecvError> {
        todo!()
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
