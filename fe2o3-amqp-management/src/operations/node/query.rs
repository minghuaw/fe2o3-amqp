use std::borrow::Cow;

use fe2o3_amqp_types::{
    messaging::{ApplicationProperties, Message},
    primitives::{OrderedMap, Value},
};

use crate::{constants::QUERY, error::Error, request::Request, response::Response};

/// A trait for handling Query request on a Manageable Node.
pub trait Query {
    /// Handles a Query request.
    fn query(&self, req: QueryRequest) -> Result<QueryResponse, Error>;
}

/// Retrieve selected attributes of Manageable Entities that can be read at this Management Node.
///
/// Since the query operation could potentially return a large number of results, this operation
/// supports pagination through which a request can specify a subset of the results to be returned.
///
/// A result set of size N can be considered to containing elements numbered from 0 to N-1. The
/// elements of the result set returned in a particular request are controlled by specifying offset
/// and count values. By setting an offset of M then only the elements numbered from M onwards will
/// be returned. If M is greater than the number of elements in the result set then no elements will
/// be returned. By additionally setting a count of C, only the elements numbered from M to
/// Min(M+C-1, N-1) will be returned. Pagination is achieved via two application-properties, offset
/// and count.
///
/// If pagination is used then it cannot be guaranteed that the result set remains consistent
/// between requests for successive pages. That is, the set of entities matching the query may have
/// changed between requests. However, stable order MUST be provided, that is, for any two queries
/// for the same parameters (except those related to pagination) then the results MUST be provided
/// in the same order. Thus, if there are no changes to the set of entities that match the query
/// then consistency MUST be maintained between requests for successive pages.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QueryRequest<'a> {
    /// If set, restricts the set of Manageable Entities requested to those that extend (directly or
    /// indirectly) the given Manageable Entity Type.
    pub entity_type: Option<Cow<'a, str>>,

    /// If set, specifies the number of the first element of the result set to be returned. If not
    /// provided, a default of 0 MUST be assumed.
    pub offset: Option<u32>,

    /// If set, specifies the number of entries from the result set to return. If not provided, all
    /// results from ‘offset’ onwards MUST be returned.
    pub count: Option<u32>,

    /// The body of the message MUST consist of an amqp-value section containing a map which MUST have
    /// the following entries, where all keys MUST be of type string:
    ///
    /// A list of strings representing the names of the attributes of the Manageable Entities being
    /// requested. The list MUST NOT contain duplicate elements. If the list contains no elements
    /// then this indicates that all attributes are being requested.
    pub attribute_names: Vec<Cow<'a, str>>,

    /// Entity type
    pub r#type: Cow<'a, str>,

    /// locales
    pub locales: Option<Cow<'a, str>>,
}

impl<'a> QueryRequest<'a> {
    /// Create a new Query request
    pub fn new(
        entity_type: impl Into<Option<Cow<'a, str>>>,
        offset: impl Into<Option<u32>>,
        count: impl Into<Option<u32>>,
        attribute_names: impl IntoIterator<Item = impl Into<Cow<'a, str>>>,
        r#type: impl Into<Cow<'a, str>>,
        locales: impl Into<Option<Cow<'a, str>>>,
    ) -> Self {
        Self {
            r#type: r#type.into(),
            locales: locales.into(),
            entity_type: entity_type.into(),
            offset: offset.into(),
            count: count.into(),
            attribute_names: attribute_names.into_iter().map(Into::into).collect(),
        }
    }
}

impl Request for QueryRequest<'_> {
    const OPERATION: &'static str = QUERY;

    type Response = QueryResponse;

    type Body = OrderedMap<String, Vec<String>>;

    fn manageable_entity_type(&mut self) -> Option<String> {
        self.entity_type.as_ref().map(|s| s.to_string())
    }

    fn locales(&mut self) -> Option<String> {
        self.locales.as_ref().map(|s| s.to_string())
    }

    fn encode_application_properties(&mut self) -> Option<ApplicationProperties> {
        let mut builder = ApplicationProperties::builder();
        if let Some(entity_type) = self.entity_type.as_ref() {
            builder = builder.insert("entityType", entity_type.to_string());
        }
        if let Some(offset) = self.offset {
            builder = builder.insert("offset", offset);
        }
        if let Some(count) = self.count {
            builder = builder.insert("count", count);
        }
        Some(builder.build())
    }

    fn encode_body(self) -> Self::Body {
        let mut map = OrderedMap::new();
        let value = self
            .attribute_names
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        map.insert(String::from("attribute_names"), value);
        map
    }
}

/// Response to a Query request.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QueryResponse {
    /// Specifies the number of entries from the result set being returned. Note that the value of count
    /// MUST be the same as number of elements in the list value associated with the results key in the
    /// body of the response message.
    pub count: u32,

    /// Body
    ///
    /// A list of strings where each element represents an attribute name. If the attributeNames
    /// passed in the body of the request contained a non-empty list then this value MUST consist of
    /// the exact same sequence of strings. If the body of the request did not contain an
    /// attributeNames entry then this value MUST contain the union of all attribute names for all
    /// Manageable Entity Types that match the query.
    pub attribute_names: Vec<String>,

    /// Body
    ///
    /// This value provides the portion of the result set being requested (as controlled by offset
    /// and count). Each element MUST provide the list of attribute values for a single Manageable
    /// Entity where the values are positionally-correlated with the names in the attributeNames
    /// entry. In the case where an attribute name is not applicable for a particular Manageable
    /// Entity then the corresponding value should be null.
    ///
    /// If the result set is empty then this value MUST be a list of zero elements.
    pub results: Vec<Vec<Value>>,
}

impl Response for QueryResponse {
    const STATUS_CODE: u16 = 200;

    type Body = OrderedMap<String, Vec<Value>>;

    type Error = Error;

    fn decode_message(mut message: Message<Self::Body>) -> Result<Self, Self::Error> {
        let count = message
            .application_properties
            .as_mut()
            .and_then(|ap| ap.swap_remove("count"))
            .map(|v| u32::try_from(v).map_err(|_| Error::DecodeError(None)))
            .ok_or(Error::DecodeError(None))??;
        let mut map = message.body;

        let attribute_names = map
            .swap_remove("attributeNames")
            .ok_or(Error::DecodeError(None))?;
        let attribute_names = attribute_names
            .into_iter()
            .map(|v| String::try_from(v).map_err(|_| Error::DecodeError(None)))
            .collect::<Result<Vec<String>, Error>>()?;

        let results = map.swap_remove("results").ok_or(Error::DecodeError(None))?;
        let results: Vec<Vec<Value>> = results
            .into_iter()
            .map(|v| match v {
                Value::List(vout) => Ok(vout),
                _ => Err(Error::DecodeError(None)),
            })
            .collect::<Result<Vec<Vec<Value>>, Error>>()?;

        Ok(Self {
            count,
            attribute_names,
            results,
        })
    }
}
