//! Implements verification for Target or Coordinator

use fe2o3_amqp_types::messaging::{Target, TargetArchetype};

use super::{ReceiverAttachError, SenderAttachError};

/// Performs verification on whether the incoming `Target` field complies with the specification
/// or meets the requirement.
pub trait VerifyTargetArchetype {
    /// Verify the `Target` field as a sender
    fn verify_as_sender(&self, other: &Self) -> Result<(), SenderAttachError>;

    /// Verify the `Target` field as a receiver
    fn verify_as_receiver(&self, other: &Self) -> Result<(), ReceiverAttachError>;
}

impl VerifyTargetArchetype for Target {
    fn verify_as_sender(&self, other: &Self) -> Result<(), SenderAttachError> {
        // The address of the source MUST be set when sent on a attach frame sent by the receiving
        // link endpoint where the dynamic flag is set to true (that is where the receiver has
        // created an addressable node at the request of the sender and is now communicating the
        // address of that created node).
        if other.dynamic && other.address.is_none() {
            Err(SenderAttachError::TargetAddressIsNoneWhenDynamicIsTrue)
        } else if !other.dynamic && other.dynamic_node_properties.is_some() {
            // If the dynamic field is not set to true this field MUST be left unset.
            Err(SenderAttachError::DynamicNodePropertiesIsSomeWhenDynamicIsFalse)
        } else {
            Ok(())
        }
    }

    fn verify_as_receiver(&self, other: &Self) -> Result<(), ReceiverAttachError> {
        // The address of the target MUST NOT be set when sent on a attach frame sent by the sending
        // link endpoint where the dynamic flag is set to true (that is where the sender is
        // requesting the receiver to create an addressable node).
        if other.dynamic && other.address.is_some() {
            Err(ReceiverAttachError::TargetAddressIsSomeWhenDynamicIsTrue)
        } else if !other.dynamic && other.dynamic_node_properties.is_some() {
            Err(ReceiverAttachError::DynamicNodePropertiesIsSomeWhenDynamicIsFalse)
        } else {
            Ok(())
        }
    }
}

pub(crate) trait VariantOfTargetArchetype {
    #[allow(dead_code)]
    fn is_coordinator(&self) -> bool;
}

impl VariantOfTargetArchetype for Target {
    fn is_coordinator(&self) -> bool {
        false
    }
}

impl VariantOfTargetArchetype for TargetArchetype {
    fn is_coordinator(&self) -> bool {
        match self {
            TargetArchetype::Target(_) => false,
            #[cfg(feature = "transaction")]
            TargetArchetype::Coordinator(_) => true,
        }
    }
}

cfg_acceptor! {
    use fe2o3_amqp_types::primitives::{Array, Symbol};

    pub(crate) trait DynamicTarget {
        fn is_dynamic(&self) -> Option<bool>;
    }

    impl DynamicTarget for Target {
        fn is_dynamic(&self) -> Option<bool> {
            Some(self.dynamic)
        }
    }

    impl DynamicTarget for TargetArchetype {
        fn is_dynamic(&self) -> Option<bool> {
            match self {
                TargetArchetype::Target(t) => t.is_dynamic(),
                #[cfg(feature = "transaction")]
                TargetArchetype::Coordinator(t) => t.is_dynamic(),
            }
        }
    }

    pub(crate) trait TargetArchetypeCapabilities {
        type Capability;

        fn capabilities_mut(&mut self) -> &mut Option<Array<Self::Capability>>;
    }

    impl TargetArchetypeCapabilities for Target {
        type Capability = Symbol;

        fn capabilities_mut(&mut self) -> &mut Option<Array<Symbol>> {
            &mut self.capabilities
        }
    }


    /// Extension trait for TargetArchetypes
    pub(crate) trait TargetArchetypeExt:
        VerifyTargetArchetype + TargetArchetypeCapabilities + VariantOfTargetArchetype + DynamicTarget
    {
    }

    impl<T> TargetArchetypeExt for T where
        T: VerifyTargetArchetype
            + TargetArchetypeCapabilities
            + VariantOfTargetArchetype
            + DynamicTarget
    {
    }
}

cfg_transaction! {
    use fe2o3_amqp_types::transaction::Coordinator;

    impl VerifyTargetArchetype for Coordinator {
        fn verify_as_sender(&self, other: &Self) -> Result<(), SenderAttachError> {
            // Note that it is the responsibility of the transaction controller to verify that the
            // capabilities of the controller meet its requirements.
            match (&self.capabilities, &other.capabilities) {
                (Some(desired), Some(provided)) => {
                    for cap in desired.0.iter() {
                        if !provided.0.contains(cap) {
                            return Err(SenderAttachError::DesireTxnCapabilitiesNotSupported);
                        }
                    }
                    Ok(())
                }
                (Some(desired), None) => {
                    if desired.0.is_empty() {
                        Ok(())
                    } else {
                        Err(SenderAttachError::DesireTxnCapabilitiesNotSupported)
                    }
                }
                (None, Some(_)) | (None, None) => Ok(()),
            }
        }

        fn verify_as_receiver(&self, _other: &Self) -> Result<(), ReceiverAttachError> {
            // Note that it is the responsibility of the transaction controller to verify that the
            // capabilities of the controller meet its requirements.
            Ok(())
        }
    }

    impl VariantOfTargetArchetype for Coordinator {
        fn is_coordinator(&self) -> bool {
            true
        }
    }

    cfg_acceptor! {
        use fe2o3_amqp_types::transaction::TxnCapability;

        impl TargetArchetypeCapabilities for Coordinator {
            type Capability = TxnCapability;

            fn capabilities_mut(&mut self) -> &mut Option<Array<TxnCapability>> {
                &mut self.capabilities
            }
        }

        impl DynamicTarget for Coordinator {
            fn is_dynamic(&self) -> Option<bool> {
                None
            }
        }
    }
}
