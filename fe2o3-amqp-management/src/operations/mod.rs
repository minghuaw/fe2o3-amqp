pub use self::{
    entity::{
        CreateRequest, CreateResponse, DeleteRequest, DeleteResponse, ReadRequest, ReadResponse,
        UpdateRequest, UpdateResponse,
    },
    // node::{
    //     DeregisterRequest, DeregisterResponse, GetAnnotationsRequest, GetAnnotationsResponse,
    //     GetAttributesRequest, GetAttributesResponse, GetMgmtNodesRequest, GetMgmtNodesResponse,
    //     GetOperationsRequest, GetOperationsResponse, GetTypesRequest, GetTypesResponse,
    //     QueryRequest, QueryResponse, RegisterRequest, RegisterResponse,
    // },
};

pub mod entity;
// pub mod node;

// pub enum Operation {
//     Create,
//     Read,
//     Update,
//     Delete,
//     Query,
//     GetTypes,
//     GetAnnotations,
//     GetAttributes,
//     GetOperations,
//     GetMgmtNodes,
//     Register,
//     Deregister,
// }

// pub enum OperationRequest {
//     Create(CreateRequest),
//     Read(ReadRequest),
//     Update(UpdateRequest),
//     Delete(DeleteRequest),
//     Query(QueryRequest),
//     GetTypes(GetTypesRequest),
//     GetAnnotations(GetAnnotationsRequest),
//     GetAttributes(GetAttributesRequest),
//     GetOperations(GetOperationsRequest),
//     GetMgmtNodes(GetMgmtNodesRequest),
//     Register(RegisterRequest),
//     Deregister(DeregisterRequest),
// }

// pub enum OperationResponse {
//     Create(CreateResponse),
//     Read(ReadResponse),
//     Update(UpdateResponse),
//     Delete(DeleteResponse),
//     Query(QueryResponse),
//     GetTypes(GetTypesResponse),
//     GetAnnotations(GetAnnotationsResponse),
//     GetAttributes(GetAttributesResponse),
//     GetOperations(GetOperationsResponse),
//     GetMgmtNodes(GetMgmtNodesResponse),
//     Register(RegisterResponse),
//     Deregister(DeregisterResponse),
// }
