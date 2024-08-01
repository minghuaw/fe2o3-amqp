use std::borrow::Cow;

use fe2o3_amqp_types::{
    messaging::Message,
    primitives::{OrderedMap, Value},
};

use crate::{constants::CREATE, error::Error, request::Request, response::Response};

/// The Create operation is used to create a new Manageable Entity.
///
/// This trait is only a placeholder for now.
pub trait Create {
    /// Handles a create operation.
    fn create(&mut self, req: CreateRequest) -> Result<CreateResponse, Error>;
}

/// A request to create a new manageable entity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CreateRequest<'a> {
    /// Additional application-properties
    ///
    /// The name of the Manageable Entity to be managed. This is case-sensitive.
    pub name: Cow<'a, str>,

    /// Entity type
    pub r#type: Cow<'a, str>,

    /// locales
    pub locales: Option<Cow<'a, str>>,

    /// The body MUST consist of an amqp-value section containing a map. The map consists of
    /// key-value pairs where the key represents the name of an attribute of the entity and the
    /// value represents the initial value it SHOULD take.
    ///
    /// The absence of an attribute name implies that the entity should take its default value, if
    /// defined.
    ///
    /// If the map contains a key-value pair where the value is null then the created entity should
    /// have no value for that attribute, overriding any default.
    ///
    /// Where the attribute value provided is of type string, but the expected AMQP type of the
    /// attribute value is not string, conversion into the correct type MUST be performed according
    /// to the following rules:
    ///
    /// - A string that consists solely of characters from the ASCII character-set, will be
    ///   converted into a symbol if so required.
    ///
    /// - A string that can be parsed as a number according to [RFC7159] will be converted to a
    ///   ubyte, ushort, uint, ulong, byte, short, int, or long if so required and the number lies
    ///   within the domain of the given AMQP type and represents an integral number
    ///
    /// - A string which can be parsed as a number according to [RFC7159] will be converted to an
    ///   float, double, decimal32, decimal64 or decimal128 if so required and the number lies within
    ///   the domain of the given AMQP type.
    ///
    /// - A string which can be parsed as true or false according to [RFC7159] will be converted to
    ///   a boolean value if so required.
    ///
    /// - A string which can be parsed as an array according to [RFC7159] will be converted into a
    ///   list (with the values type-converted into elements as necessary according to the same rules)
    ///   if so required.
    ///
    /// - A string which can be parsed as an object according to [RFC7159] will be converted into a
    ///   map (with the values type-converted into map values as necessary according to the same
    ///   rules) if so required.
    pub body: OrderedMap<String, Value>,
}

impl<'a> CreateRequest<'a> {
    /// Creates a new CreateRequest.
    pub fn new(
        name: impl Into<Cow<'a, str>>,
        r#type: impl Into<Cow<'a, str>>,
        locales: Option<impl Into<Cow<'a, str>>>,
        body: OrderedMap<String, Value>,
    ) -> Self {
        Self {
            name: name.into(),
            r#type: r#type.into(),
            locales: locales.map(|x| x.into()),
            body,
        }
    }
}

impl<'a> Request for CreateRequest<'a> {
    const OPERATION: &'static str = CREATE;

    type Response = CreateResponse;

    type Body = OrderedMap<String, Value>;

    fn manageable_entity_type(&mut self) -> Option<String> {
        Some(self.r#type.to_string())
    }

    fn locales(&mut self) -> Option<String> {
        self.locales.as_ref().map(|s| s.to_string())
    }

    fn encode_body(self) -> Self::Body {
        self.body
    }
}

/// If the request was successful then the statusCode MUST be 201 (Created) and the body of the
/// message MUST consist of an amqp-value section that contains a map containing the actual
/// attributes of the entity created. These MAY differ from those requested in two ways:
///
/// - Default values may be returned for values not specified
/// - Specific/concrete values may be returned for generic/base values specified
/// - The value associated with an attribute may have been converted into the correct amqp type
///
/// (e.g. the string “2” into the integer value 2) A map containing attributes that are not
/// applicable for the entity being created, or invalid values for a given attribute, MUST result in
/// a failure response with a statusCode of 400 (Bad Request).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CreateResponse {
    /// The body of Create response.
    pub entity_attributes: OrderedMap<String, Value>,
}

impl Response for CreateResponse {
    const STATUS_CODE: u16 = 201;

    type Body = Option<OrderedMap<String, Value>>;

    type Error = Error;

    fn decode_message(message: Message<Self::Body>) -> Result<Self, Self::Error> {
        match message.body {
            Some(map) => Ok(Self {
                entity_attributes: map,
            }),
            None => Ok(Self {
                entity_attributes: OrderedMap::with_capacity(0),
            }),
        }
    }
}
