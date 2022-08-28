use fe2o3_amqp_types::primitives::Value;

use crate::request::MessageSerializer;

use self::{entity::{CreateRequest, ReadRequest, UpdateRequest, DeleteRequest, CreateResponse, ReadResponse, UpdateResponse, DeleteResponse}, node::{QueryRequest, GetTypesRequest, GetAnnotationsRequest, GetAttributesRequest, GetMgmtNodesRequest, GetOperationsRequest, RegisterRequest, DeregisterRequest, QueryResponse, GetTypesResponse, GetAnnotationsResponse, GetAttributesResponse, GetOperationsResponse, GetMgmtNodesResponse, RegisterResponse, DeregisterResponse}};

pub mod entity;
pub mod node;

pub const OPERATION: &str = "operation";
pub const CREATE: &str = "CREATE";
pub const READ: &str = "READ";
pub const UPDATE: &str = "UPDATE";
pub const DELETE: &str = "DELETE";
pub const QUERY: &str = "QUERY";
pub const GET_TYPES: &str = "GET-TYPES";
pub const GET_ANNOTATIONS: &str = "GET-ANNOTATIONS";
pub const GET_ATTRIBUTES: &str = "GET-ATTRIBUTES";
pub const GET_OPERATIONS: &str = "GET-OPERATIONS";
pub const GET_MGMT_NODES: &str = "GET-MGMT-NODES";
pub const REGISTER: &str = "REGISTER";
pub const DEREGISTER: &str = "DEREGISTER";

pub enum OperationRequest {
    Create(CreateRequest),
    Read(ReadRequest),
    Update(UpdateRequest),
    Delete(DeleteRequest),
    Query(QueryRequest),
    GetTypes(GetTypesRequest),
    GetAnnotations(GetAnnotationsRequest),
    GetAttributes(GetAttributesRequest),
    GetOperations(GetOperationsRequest),
    GetMgmtNodes(GetMgmtNodesRequest),
    Register(RegisterRequest),
    Deregister(DeregisterRequest),
}

pub enum OperationResponse {
    Create(CreateResponse),
    Read(ReadResponse),
    Update(UpdateResponse),
    Delete(DeleteResponse),
    Query(QueryResponse),
    GetTypes(GetTypesResponse),
    GetAnnotations(GetAnnotationsResponse),
    GetAttributes(GetAttributesResponse),
    GetOperations(GetOperationsResponse),
    GetMgmtNodes(GetMgmtNodesResponse),
    Register(RegisterResponse),
    Deregister(DeregisterResponse),
}