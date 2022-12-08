//! Defines constants used in the AMQP Management protocol.

/// OPERATION key
pub const OPERATION: &str = "operation";

/// CREATE operation
pub const CREATE: &str = "CREATE";

/// READ operation
pub const READ: &str = "READ";

/// UPDATE operation
pub const UPDATE: &str = "UPDATE";

/// DELETE operation
pub const DELETE: &str = "DELETE";

/// QUERY operation
pub const QUERY: &str = "QUERY";

/// GET_TYPES operation
pub const GET_TYPES: &str = "GET-TYPES";

/// GET_ANNOTATIONS operation
pub const GET_ANNOTATIONS: &str = "GET-ANNOTATIONS";

/// GET_ATTRIBUTES operation
pub const GET_ATTRIBUTES: &str = "GET-ATTRIBUTES";

/// GET_OPERATIONS operation
pub const GET_OPERATIONS: &str = "GET-OPERATIONS";

/// GET_MGMT_NODES operation
pub const GET_MGMT_NODES: &str = "GET-MGMT-NODES";

/// REGISTER operation
pub const REGISTER: &str = "REGISTER";

/// DEREGISTER operation
pub const DEREGISTER: &str = "DEREGISTER";

/// NAME key
pub const NAME: &str = "name";

/// IDENTITY key
pub const IDENTITY: &str = "identity";

/// TYPE key
pub const TYPE: &str = "type";

/// LOCALES key
pub const LOCALES: &str = "locales";

/// ENTITY_TYPE key
pub const ENTITY_TYPE: &str = "entityType";

pub(crate) mod kebab_case {
    pub const STATUS_CODE: &str = "status-code";
    pub const STATUS_DESCRIPTION: &str = "status-description";
}

pub(crate) mod lower_camel_case {
    pub const STATUS_CODE: &str = "statusCode";
    pub const STATUS_DESCRIPTION: &str = "statusDescription";
}
