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

pub const NAME: &str = "name";
pub const IDENTITY: &str = "identity";
pub const TYPE: &str = "type";
pub const LOCALES: &str = "locales";
pub const ENTITY_TYPE: &str = "entityType";

pub(crate) mod kebab_case {
    pub const STATUS_CODE: &str = "status-code";
    pub const STATUS_DESCRIPTION: &str = "status-description";
}

pub(crate) mod lower_camel_case {
    pub const STATUS_CODE: &str = "statusCode";
    pub const STATUS_DESCRIPTION: &str = "statusDescription";
}
