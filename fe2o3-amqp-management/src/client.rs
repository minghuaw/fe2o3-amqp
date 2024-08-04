//! Implements a client for the AMQP 1.0 management working draft.

use fe2o3_amqp::{
    link::{
        DetachError, DetachThenResumeReceiverError, ReceiverAttachExchange,
        ReceiverResumeErrorKind, SendError,
    },
    session::SessionHandle,
    Delivery, Receiver, Sender,
};
use fe2o3_amqp_types::{
    definitions::Fields,
    messaging::{Body, FromBody, IntoBody, MessageId, Outcome, Properties},
    primitives::Value,
};

use crate::{
    error::{AttachError, DetachThenResumeError, Error},
    request::Request,
    response::Response,
    DEFAULT_CLIENT_NODE_ADDRESS, MANAGEMENT_NODE_ADDRESS,
};

/// A client for the AMQP 1.0 management working draft. It contains a sender and receiver link.
#[derive(Debug)]
pub struct MgmtClient {
    req_id: u64,
    client_node_addr: String,
    sender: Sender,
    receiver: Receiver,
}

impl MgmtClient {
    /// Creates a builder for a management client.
    pub fn builder() -> MgmtClientBuilder {
        MgmtClientBuilder::default()
    }

    /// Attach a management client to a session.
    pub async fn attach<R>(
        session: &mut SessionHandle<R>,
        client_node_addr: impl Into<String>,
    ) -> Result<Self, AttachError> {
        Self::builder()
            .client_node_addr(client_node_addr)
            .attach(session)
            .await
    }

    /// Detach and then resume the management client on a session.
    pub async fn detach_then_resume_on_session<R>(
        &mut self,
        session: &SessionHandle<R>,
    ) -> Result<(), DetachThenResumeError> {
        self.sender.detach_then_resume_on_session(session).await?;
        while let ReceiverAttachExchange::IncompleteUnsettled =
            self.receiver.detach_then_resume_on_session(session).await?
        {
            match self.receiver.recv::<Body<Value>>().await {
                Ok(delivery) => {
                    self.receiver.reject(&delivery, None).await.map_err(|e| {
                        let err = ReceiverResumeErrorKind::FlowError(e);
                        let err = DetachThenResumeReceiverError::Resume(err);
                        DetachThenResumeError::Receiver(err)
                    })?;
                }
                Err(_e) => {
                    #[cfg(feature = "log")]
                    log::error!("Error receiving message while resuming receiver {}", _e);
                    #[cfg(feature = "tracing")]
                    tracing::error!("Error receiving message while resuming receiver {}", _e);
                }
            }
        }
        Ok(())
    }

    /// Close/detach the management client.
    pub async fn close(self) -> Result<(), DetachError> {
        self.sender.close().await?;
        self.receiver.close().await?;
        Ok(())
    }

    /// Send a request and wait for the outcome.
    ///
    /// This currently takes ownership of the request because it needs to set the request id if the field is not set.
    pub async fn send_request(&mut self, request: impl Request) -> Result<Outcome, SendError> {
        let mut message = request.into_message().map_body(IntoBody::into_body);

        // Only insert the request-id if it's not already set
        let properties = message.properties.get_or_insert(Properties::default());
        properties.message_id.get_or_insert({
            let message_id = MessageId::from(self.req_id);
            self.req_id = self.req_id.wrapping_add(1);
            message_id
        });
        properties
            .reply_to
            .get_or_insert(self.client_node_addr.clone());

        self.sender.send(message).await
    }

    /// Receive a response.
    pub async fn recv_response<Res>(&mut self) -> Result<Res, Error>
    where
        Res: Response,
        Res::Error: Into<Error>,
        for<'de> Res::Body: FromBody<'de> + std::fmt::Debug + Send,
    {
        let delivery: Delivery<Res::Body> = self.receiver.recv().await?;
        self.receiver.accept(&delivery).await?;

        Res::from_message(delivery.into_message()).map_err(Into::into)
    }

    /// Send a request and receive a response.
    pub async fn call<Req, Res>(&mut self, request: Req) -> Result<Res, Error>
    where
        Req: Request<Response = Res>,
        Res: Response,
        Res::Error: Into<Error>,
        for<'de> Res::Body: FromBody<'de> + std::fmt::Debug + Send,
    {
        let outcome = self.send_request(request).await?;
        let _accepted = outcome.accepted_or_else(Error::NotAccepted)?;
        self.recv_response().await
    }
}

/// A builder for a management client.
#[derive(Debug)]
pub struct MgmtClientBuilder {
    mgmt_node_addr: String,
    client_node_addr: String,
    sender_properties: Option<Fields>,
    receiver_properties: Option<Fields>,
}

impl Default for MgmtClientBuilder {
    fn default() -> Self {
        MgmtClientBuilder {
            mgmt_node_addr: String::from(MANAGEMENT_NODE_ADDRESS),
            client_node_addr: String::from(DEFAULT_CLIENT_NODE_ADDRESS),
            sender_properties: None,
            receiver_properties: None,
        }
    }
}

impl MgmtClientBuilder {
    /// Set the sender link properties.
    pub fn sender_properties(mut self, properties: Fields) -> Self {
        self.sender_properties = Some(properties);
        self
    }

    /// Set the receiver link properties.
    pub fn receiver_properties(mut self, properties: Fields) -> Self {
        self.receiver_properties = Some(properties);
        self
    }

    /// Set the management node address.
    pub fn management_node_address(mut self, mgmt_node_addr: impl Into<String>) -> Self {
        self.mgmt_node_addr = mgmt_node_addr.into();
        self
    }

    /// Set the client node address.
    pub fn client_node_addr(mut self, client_node_addr: impl Into<String>) -> Self {
        self.client_node_addr = client_node_addr.into();
        self
    }

    /// Attach a management client to a session.
    pub async fn attach<R>(
        self,
        session: &mut SessionHandle<R>,
    ) -> Result<MgmtClient, AttachError> {
        let mut sender_builder = Sender::builder()
            .name(format!("{}-mgmt-sender", self.client_node_addr))
            .target(Some(&self.mgmt_node_addr));
        if let Some(properties) = self.sender_properties {
            sender_builder = sender_builder.properties(properties);
        }
        let sender = sender_builder.attach(session).await?;

        let mut receiver_builder = Receiver::builder()
            .name(format!("{}-mgmt-receiver", self.client_node_addr))
            .source(Some(self.mgmt_node_addr))
            .target(Some(&self.client_node_addr));
        if let Some(properties) = self.receiver_properties {
            receiver_builder = receiver_builder.properties(properties);
        }
        let receiver = receiver_builder.attach(session).await?;

        Ok(MgmtClient {
            req_id: 0,
            client_node_addr: self.client_node_addr,
            sender,
            receiver,
        })
    }
}
