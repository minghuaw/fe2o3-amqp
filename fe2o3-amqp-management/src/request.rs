use fe2o3_amqp_types::messaging::{IntoBody, Message};

use crate::response::Response;

use fe2o3_amqp_types::{messaging::{
    ApplicationProperties, DeliveryAnnotations, Footer, Header, 
    MessageAnnotations, Properties,
}, primitives::SimpleValue};

use crate::{constants};

pub trait Request: Sized {
    /// Management operation
    const OPERATION: &'static str;

    type Response: Response;
    type Body: IntoBody;

    fn locales(&mut self) -> Option<String> {
        None
    }

    /// Set the manageable entity type.
    /// 
    /// This seems to be mandatory for all requests in the working draft. However, existing
    /// implementations do not seem to comply, which is why it is an option.
    fn manageable_entity_type(&mut self) -> Option<String> {
        None
    }

    fn encode_header(&mut self) -> Option<Header> {
        None
    }

    fn encode_delivery_annotations(&mut self) -> Option<DeliveryAnnotations> {
        None
    }

    fn encode_message_annotations(&mut self) -> Option<MessageAnnotations> {
        None
    }

    fn encode_properties(&mut self) -> Option<Properties> {
        None
    }

    fn encode_application_properties(&mut self) -> Option<ApplicationProperties> {
        None
    }

    fn encode_body(&mut self) -> Self::Body;

    fn encode_footer(&mut self) -> Option<Footer> {
        None
    }

    fn into_message(mut self) -> Message<Self::Body> {
        let mut application_properties = self.encode_application_properties().unwrap_or_default();
        application_properties.as_inner_mut()
            .entry(constants::OPERATION.to_string())
            .or_insert(SimpleValue::String(Self::OPERATION.to_string()));
        if let Some(entity_type) = self.manageable_entity_type() {
            application_properties.as_inner_mut()
                .entry(constants::TYPE.to_string())
                .or_insert(SimpleValue::String(entity_type));
        }
        if let Some(locales) = self.locales() {
            application_properties.as_inner_mut()
                .entry(constants::LOCALES.to_string())
                .or_insert(SimpleValue::String(locales));
        }

        Message {
            header: self.encode_header(),
            delivery_annotations: self.encode_delivery_annotations(),
            message_annotations: self.encode_message_annotations(),
            properties: self.encode_properties(),
            application_properties: Some(application_properties),
            body: self.encode_body(),
            footer: self.encode_footer(),
        }
    }
}
