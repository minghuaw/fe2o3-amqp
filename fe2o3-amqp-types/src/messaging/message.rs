use serde::{Serialize, Deserialize};

use super::{ApplicationProperties, Data, DeliveryAnnotations, Footer, Header, MessageAnnotations, Properties};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    header: Header,
    delivery_annotations: DeliveryAnnotations,
    message_annotations: MessageAnnotations,
    properties: Properties,
    application_properties: ApplicationProperties,
    data: Data,
    footer: Footer
}