use serde::{Deserialize, Serialize};

use super::{
    ApplicationProperties, Data, DeliveryAnnotations, Footer, Header, MessageAnnotations,
    Properties,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub header: Header,
    pub delivery_annotations: DeliveryAnnotations,
    pub message_annotations: MessageAnnotations,
    pub properties: Properties,
    pub application_properties: ApplicationProperties,
    pub data: Data,
    pub footer: Footer,
}


pub enum MessageBody {

}

