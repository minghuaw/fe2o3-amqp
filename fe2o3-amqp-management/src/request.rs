use fe2o3_amqp_types::messaging::{Message, IntoSerializableBody};

// use crate::constantsOperationRequest;

// pub struct RequestMessageProperties {
//     /// The management operation to be performed. This is case-sensitive.
//     pub opertaion: String,

//     /// The Manageable Entity Type of the Manageable Entity to be managed. This is case-sensitive.
//     pub mgmt_entity_type: String,

//     /// A listing of locales that the sending peer permits for incoming informational text in
//     /// response messages. The value MUST be of the form (presented in the augmented BNF defined in
//     /// section 2 of [RFC2616]):
//     ///
//     /// #Language-Tag
//     ///
//     /// where Language-Tag is defined in [BCP47]. That is a sequence of language tags separated by
//     /// one or more commas and OPTIONAL linear white space, such as “de-CH, it-CH, fr-CH”
//     ///
//     /// This locales MUST be ordered in decreasing level of preference. The receiving partner will
//     /// choose the first (most preferred) incoming locale from those which it supports. If none of
//     /// the requested locales are supported, "en-US" MUST be chosen. Note that "en-US" need not be
//     /// supplied in the locales as it is always the fallback.
//     ///
//     /// The string is not case-sensitive.
//     pub locales: Option<String>,
// }

// pub struct Request {
//     pub operation: OperationRequest,
//     pub mgmt_entity_type: String,
//     pub locales: Option<String>,
// }

pub trait MessageSerializer {
    type Body: IntoSerializableBody;

    fn into_message(self) -> Message<Self::Body>;
}
