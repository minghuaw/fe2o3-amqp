use std::borrow::Cow;

use fe2o3_amqp_types::messaging::Message;

use crate::{constants::GET_MGMT_NODES, error::Error, request::Request, response::Response};

use super::get::GetRequest;

pub trait GetMgmtNodes {
    fn get_mgmt_nodes(&self, req: GetMgmtNodesRequest) -> Result<GetMgmtNodesResponse, Error>;
}

/// GET-MGMT-NODES
///
/// Retrieve the list of addresses of other Management Nodes which this Management Node is aware of.
///
/// Body:
///
/// No information is carried in the message body therefore any message body is valid and MUST be ignored.
pub struct GetMgmtNodesRequest<'a> {
    inner: GetRequest<'a>,
}

impl<'a> GetMgmtNodesRequest<'a> {
    pub fn new(r#type: impl Into<Cow<'a, str>>, locales: Option<impl Into<Cow<'a, str>>>) -> Self {
        Self {
            inner: GetRequest::new(None, r#type, locales),
        }
    }
}

impl<'a> Request for GetMgmtNodesRequest<'a> {
    const OPERATION: &'static str = GET_MGMT_NODES;

    type Response = GetMgmtNodesResponse;

    type Body = ();

    fn manageable_entity_type(&mut self) -> Option<String> {
        self.inner.manageable_entity_type()
    }

    fn locales(&mut self) -> Option<String> {
        self.inner.locales()
    }

    fn encode_application_properties(
        &mut self,
    ) -> Option<fe2o3_amqp_types::messaging::ApplicationProperties> {
        self.inner.encode_application_properties()
    }

    fn encode_body(self) -> Self::Body {
        
    }
}

/// If the request was successful then the statusCode MUST be 200 (OK) and the body of the message
/// MUST consist of an amqp-value section containing a list of addresses of other Management Nodes
/// known by this Management Node (each element of the list thus being a string). If no other
/// Management Nodes are known then the amqp-value section MUST contain a list of zero elements.
pub struct GetMgmtNodesResponse {
    pub addresses: Vec<String>,
}

impl GetMgmtNodesResponse {}

impl Response for GetMgmtNodesResponse {
    const STATUS_CODE: u16 = 200;

    type Body = Option<Vec<String>>;

    type Error = Error;

    fn decode_message(message: Message<Self::Body>) -> Result<Self, Self::Error> {
        match message.body {
            Some(addresses) => Ok(Self { addresses }),
            None => Ok(Self {
                addresses: Vec::with_capacity(0),
            }),
        }
    }
}
