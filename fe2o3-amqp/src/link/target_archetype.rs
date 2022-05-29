//! Implements verification for Target or Coordinator

use fe2o3_amqp_types::{
    definitions::{self, AmqpError},
    messaging::Target,
};

#[cfg(feature = "transaction")]
use fe2o3_amqp_types::transaction::Coordinator;

/// Performs verification on whether the incoming `Target` field complies with the specification
/// or meets the requirement.
pub trait VerifyTargetArchetype {
    fn verify_as_sender(&self, other: &Self) -> Result<(), definitions::Error>;
    fn verify_as_receiver(&self, other: &Self) -> Result<(), definitions::Error>;
}

impl VerifyTargetArchetype for Target {
    fn verify_as_sender(&self, other: &Self) -> Result<(), definitions::Error> {
        // The address of the source MUST be set when sent on a attach frame sent by the receiving
        // link endpoint where the dynamic flag is set to true (that is where the receiver has
        // created an addressable node at the request of the sender and is now communicating the
        // address of that created node).
        if other.dynamic && other.address.is_none() {
            return Err(definitions::Error::new(
                AmqpError::NotAllowed,
                "The address of the source MUST be set when sent on a attach frame sent by the receiving link endpoint where the dynamic flag is set to true".to_string(),
                None
            ));
        } else if !other.dynamic && other.dynamic_node_properties.is_some() {
            // If the dynamic field is not set to true this field MUST be left unset.
            return Err(definitions::Error::new(
                AmqpError::NotAllowed,
                "If the dynamic field is not set to true this field MUST be left unset."
                    .to_string(),
                None,
            ));
        }

        Ok(())
    }

    fn verify_as_receiver(&self, other: &Self) -> Result<(), definitions::Error> {
        // The address of the target MUST NOT be set when sent on a attach frame sent by the sending
        // link endpoint where the dynamic flag is set to true (that is where the sender is
        // requesting the receiver to create an addressable node).
        if other.dynamic && other.address.is_some() {
            return Err(definitions::Error::new(
                AmqpError::NotAllowed,
                "The address of the target MUST NOT be set when sent on a attach frame sent by the sending link endpoint where the dynamic flag is set to true".to_string(),
                None
            ));
        } else if !other.dynamic && other.dynamic_node_properties.is_some() {
            // If the dynamic field is not set to true this field MUST be left unset.
            return Err(definitions::Error::new(
                AmqpError::NotAllowed,
                "If the dynamic field is not set to true this field MUST be left unset."
                    .to_string(),
                None,
            ));
        }

        Ok(())
    }
}

#[cfg(feature = "transaction")]
impl VerifyTargetArchetype for Coordinator {
    fn verify_as_sender(&self, other: &Self) -> Result<(), definitions::Error> {
        // Note that it is the responsibility of the transaction controller to verify that the
        // capabilities of the controller meet its requirements.
        match (&self.capabilities, &other.capabilities) {
            (Some(desired), Some(provided)) => {
                for cap in desired {
                    if !provided.contains(cap) {
                        return Err(definitions::Error::new(
                            AmqpError::InternalError,
                            "Desired transaction capabilities are not supported".to_string(),
                            None,
                        ));
                    }
                }
                Ok(())
            }
            (Some(desired), None) => {
                if desired.is_empty() {
                    Ok(())
                } else {
                    Err(definitions::Error::new(
                        AmqpError::InternalError,
                        "Desired transaction capabilities are not supported".to_string(),
                        None,
                    ))
                }
            }
            (None, Some(_)) | (None, None) => Ok(()),
        }
    }

    fn verify_as_receiver(&self, _other: &Self) -> Result<(), definitions::Error> {
        // Note that it is the responsibility of the transaction controller to verify that the
        // capabilities of the controller meet its requirements.
        Ok(())
    }
}
