//! Retrieve selected attributes of Manageable Entities that can be read at this Management Node.
//! 
//! Since the query operation could potentially return a large number of results, this operation
//! supports pagination through which a request can specify a subset of the results to be returned.
//! 
//! A result set of size N can be considered to containing elements numbered from 0 to N-1. The
//! elements of the result set returned in a particular request are controlled by specifying offset
//! and count values. By setting an offset of M then only the elements numbered from M onwards will
//! be returned. If M is greater than the number of elements in the result set then no elements will
//! be returned. By additionally setting a count of C, only the elements numbered from M to
//! Min(M+C-1, N-1) will be returned. Pagination is achieved via two application-properties, offset
//! and count.
//! 
//! If pagination is used then it cannot be guaranteed that the result set remains consistent
//! between requests for successive pages. That is, the set of entities matching the query may have
//! changed between requests. However, stable order MUST be provided, that is, for any two queries
//! for the same parameters (except those related to pagination) then the results MUST be provided
//! in the same order. Thus, if there are no changes to the set of entities that match the query
//! then consistency MUST be maintained between requests for successive pages.

pub struct QueryRequestProperties {
    /// If set, restricts the set of Manageable Entities requested to those that extend (directly or
    /// indirectly) the given Manageable Entity Type.
    entity_type: Option<String>,

    /// If set, specifies the number of the first element of the result set to be returned. If not
    /// provided, a default of 0 MUST be assumed.
    offset: Option<u32>,

    /// If set, specifies the number of entries from the result set to return. If not provided, all
    /// results from ‘offset’ onwards MUST be returned.
    count: Option<u32>,
}

/// The body of the message MUST consist of an amqp-value section containing a map which MUST have
/// the following entries, where all keys MUST be of type string:
pub struct QueryRequestBody {
    /// A list of strings representing the names of the attributes of the Manageable Entities being
    /// requested. The list MUST NOT contain duplicate elements. If the list contains no elements
    /// then this indicates that all attributes are being requested.
    attribute_names: Vec<String>,
}

pub struct QueryResponseProperties {

}

pub struct QueryResponse {
    properties: QueryRequestProperties,
    
}

impl QueryResponse {
    const STATUS_CODE: u16 = 200;
}