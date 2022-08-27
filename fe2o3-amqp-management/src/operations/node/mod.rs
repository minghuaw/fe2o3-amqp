//! A Management Node Operation is an operation directed to the Management Node itself rather than an entity it is managing.
//! Of the standard application-properties (see Section 3.1), name MUST be provided with a value of “self”, type MUST be provided with a value of “org.amqp.management” and identity MUST NOT be provided.
//! The following Management Node Operations SHOULD be supported:
//! • QUERY
//! • GET-TYPES
//! • GET-ANNOTATIONS
//! • GET-ATTRIBUTES
//! • GET-OPERATIONS
//! • GET-MGMT-NODES
//! The following Management Node Operations MAY be supported:
//! • REGISTER
//! • DEREGISTER

mod query;
mod get_types;
mod get_annotations;
mod get_attributes;
mod get_operations;
mod get_mgmt_nodes;
mod register;
mod deregister;
