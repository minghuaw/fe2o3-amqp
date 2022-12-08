//! Defines the Request trait for AMQP 1.0 management requests.

use fe2o3_amqp_types::messaging::{IntoBody, Message};

use crate::response::Response;

use fe2o3_amqp_types::{
    messaging::{
        ApplicationProperties, DeliveryAnnotations, Footer, Header, MessageAnnotations, Properties,
    },
    primitives::SimpleValue,
};

use crate::constants;

/// A trait for AMQP 1.0 management requests.
pub trait Request: Sized {
    /// Management operation
    const OPERATION: &'static str;

    /// The response type for this request.
    type Response: Response;

    /// The body type for this request.
    type Body: IntoBody;

    /// Set the locales for this request.
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

    /// Encode the Header section of the message.
    fn encode_header(&mut self) -> Option<Header> {
        None
    }

    /// Encode the DeliveryAnnotations section of the message.
    fn encode_delivery_annotations(&mut self) -> Option<DeliveryAnnotations> {
        None
    }

    /// Encode the MessageAnnotations section of the message.
    fn encode_message_annotations(&mut self) -> Option<MessageAnnotations> {
        None
    }

    /// Encode the Properties section of the message.
    fn encode_properties(&mut self) -> Option<Properties> {
        None
    }

    /// Encode the ApplicationProperties section of the message.
    fn encode_application_properties(&mut self) -> Option<ApplicationProperties> {
        None
    }

    /// Encode the body of the message.
    /// 
    /// This is the only function that the user needs to implement if the request message
    /// doesn't encode other message sections.
    fn encode_body(self) -> Self::Body;

    /// Encode the Footer section of the message.
    fn encode_footer(&mut self) -> Option<Footer> {
        None
    }

    /// Encode this request into a message.
    fn into_message(mut self) -> Message<Self::Body> {
        let header = self.encode_header();
        let delivery_annotations = self.encode_delivery_annotations();
        let message_annotations = self.encode_message_annotations();
        let properties = self.encode_properties();

        let mut application_properties = self.encode_application_properties().unwrap_or_default();
        application_properties
            .as_inner_mut()
            .entry(constants::OPERATION.to_string())
            .or_insert(SimpleValue::String(Self::OPERATION.to_string()));
        if let Some(entity_type) = self.manageable_entity_type() {
            application_properties
                .as_inner_mut()
                .entry(constants::TYPE.to_string())
                .or_insert(SimpleValue::String(entity_type));
        }
        if let Some(locales) = self.locales() {
            application_properties
                .as_inner_mut()
                .entry(constants::LOCALES.to_string())
                .or_insert(SimpleValue::String(locales));
        }

        let footer = self.encode_footer();

        // `encode_body` will consume self, so we need to call it last.
        let body = self.encode_body();

        Message::builder()
            .header(header)
            .delivery_annotations(delivery_annotations)
            .message_annotations(message_annotations)
            .properties(properties)
            .application_properties(application_properties)
            .body(body)
            .footer(footer)
            .build()
    }
}
