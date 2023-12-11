//! Defines and implements traits to verify source capabilities

use fe2o3_amqp_types::messaging::{FilterSet, Source};

use super::{DesiredFilterNotSupported, ReceiverAttachError, SenderAttachError};

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
        // The receiving endpoint sets its desired filter, the sending endpoint
        // sets the filter actually in place (including any filters defaulted at the node). The receiving endpoint
        // MUST check that the filter in place meets its needs and take responsibility for detaching if it does
        // not.
        //
        // This does NOT check if the value is the same because some brokers uses the draft version of the
        // spec where the value is not a described type.
        verify_filter(&self.filter, &other.filter)?;

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

fn verify_filter(
    desired: &Option<FilterSet>,
    supported: &Option<FilterSet>,
) -> Result<(), DesiredFilterNotSupported> {
    use fe2o3_amqp_types::primitives::Value;

    let mut result = Ok(());

    let desired = match desired {
        Some(desired) => desired,
        None => return result,
    };

    for (key, _value) in desired.iter() {
        if let Some(Value::Null) | None = supported.as_ref().and_then(|m| m.get(key)) {
            match &mut result {
                Ok(_) => {
                    result = Err(DesiredFilterNotSupported {
                        not_supported: vec![key.clone()],
                    })
                }
                Err(e) => e.not_supported.push(key.clone()),
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::verify_filter;

    use fe2o3_amqp_ext::filters;
    use fe2o3_amqp_types::messaging::FilterSet;
    use serde_amqp::Value;

    #[test]
    fn empty_desired_and_empty_supported_returns_ok() {
        let desired = None;
        let supported = None;

        assert!(verify_filter(&desired, &supported).is_ok());
    }

    #[test]
    fn desired_filter_is_not_found_in_empty_supported_filter_set_returns_err() {
        let filter = filters::LegacyAmqpDirectBinding("DESIRED".to_string());

        let mut desired = FilterSet::new();
        desired.insert(
            filters::LegacyAmqpDirectBinding::descriptor_name(),
            Value::Described(Box::new(filter.into())),
        );

        let supported = None;

        assert!(verify_filter(&Some(desired), &supported).is_err());
    }

    #[test]
    fn desired_filter_is_not_found_in_non_empty_supported_filter_set_returns_err() {
        let filter = filters::LegacyAmqpDirectBinding("DESIRED".to_string());

        let mut desired = FilterSet::new();
        desired.insert(
            filters::LegacyAmqpDirectBinding::descriptor_name(),
            Value::Described(Box::new(filter.into())),
        );

        let filter = filters::LegacyAmqpTopicBinding("SUPPORTED".to_string());

        let mut supported = FilterSet::new();
        supported.insert(
            filters::LegacyAmqpTopicBinding::descriptor_name(),
            Value::Described(Box::new(filter.into())),
        );

        assert!(verify_filter(&Some(desired), &Some(supported)).is_err());
    }

    #[test]
    fn desired_filter_is_null_valued_in_supported_filter_set_returns_err() {
        let filter = filters::LegacyAmqpDirectBinding("DESIRED".to_string());

        let mut desired = FilterSet::new();
        desired.insert(
            filters::LegacyAmqpDirectBinding::descriptor_name(),
            Value::Described(Box::new(filter.into())),
        );

        let mut supported = FilterSet::new();
        supported.insert(
            filters::LegacyAmqpDirectBinding::descriptor_name(),
            Value::Null,
        );

        assert!(verify_filter(&Some(desired), &Some(supported)).is_err());
    }

    #[test]
    fn desired_filter_is_found_in_supported_filter_set_returns_ok() {
        let filter = filters::LegacyAmqpDirectBinding("DESIRED".to_string());

        let mut desired = FilterSet::new();
        desired.insert(
            filters::LegacyAmqpDirectBinding::descriptor_name(),
            Value::Described(Box::new(filter.into())),
        );

        let filter = filters::LegacyAmqpDirectBinding("DESIRED".to_string());

        let mut supported = FilterSet::new();
        supported.insert(
            filters::LegacyAmqpDirectBinding::descriptor_name(),
            Value::Described(Box::new(filter.into())),
        );

        assert!(verify_filter(&Some(desired), &Some(supported)).is_ok());
    }
}
