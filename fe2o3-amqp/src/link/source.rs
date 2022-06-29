//! Defines and implements traits to verify source capabilities

use fe2o3_amqp_types::messaging::Source;

use super::{ReceiverAttachError, SenderAttachError};

pub trait VerifySource {
    fn verify_as_sender(&self, other: &Self) -> Result<(), SenderAttachError>;
    fn verify_as_receiver(&self, other: &Self) -> Result<(), ReceiverAttachError>;
}

impl VerifySource for Source {
    fn verify_as_sender(&self, other: &Self) -> Result<(), SenderAttachError> {
        if other.dynamic && other.address.is_some() {
            // When set to true by the receiving link endpoint, this field constitutes a request for the sending
            // peer to dynamically create a node at the source. In this case the address field MUST NOT be set
            Err(SenderAttachError::SourceAddressIsSomeWhenDynamicIsTrue)
        } else if !other.dynamic && other.dynamic_node_properties.is_some() {
            // If the dynamic field is not set to true this field MUST be left unset.
            Err(SenderAttachError::DynamicNodePropertiesIsSomeWhenDynamicIsFalse)
        } else {
            // TODO: verify the capabilities?
            Ok(())
        }
    }

    fn verify_as_receiver(&self, other: &Self) -> Result<(), ReceiverAttachError> {
        if other.dynamic && other.address.is_none() {
            // When set to true by the sending link endpoint this field indicates creation of a dynamically created
            // node. In this case the address field will contain the address of the created node
            Err(ReceiverAttachError::SourceAddressIsNoneWhenDynamicIsTrue)
        } else if !other.dynamic && other.dynamic_node_properties.is_some() {
            Err(ReceiverAttachError::DynamicNodePropertiesIsSomeWhenDynamicIsFalse)
        } else {
            // TODO: verify the capabilities?
            Ok(())
        }
    }
}
