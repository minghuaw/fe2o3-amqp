use std::collections::BTreeMap;
use serde::{Serialize, Deserialize};
use fe2o3_amqp::{types::Symbol, value::Value};

/// 3.5.8 Filter Set
/// <type name="filter-set" class="restricted" source="map"/>
///
/// A set of named filters. Every key in the map MUST be of type symbol, 
/// every value MUST be either null or of a described type which provides 
/// the archetype filter. A filter acts as a function on a message which 
/// returns a boolean result indicating whether the message can pass through
/// that filter or not. A message will pass through a filter-set if and only 
/// if it passes through each of the named filters. If the value for a given 
/// key is null, this acts as if there were no such key present 
/// (i.e., all messages pass through the null filter).
/// Filter types are a defined extension point. The filter types that a given 
/// source supports will be indicated by the capabilities of the source. 
/// A registry of commonly defined filter types and their capabilities is 
/// maintained [AMQPFILTERS].
///
/// TODO: add described type representation in Value
#[derive(Debug, Serialize, Deserialize)]
pub struct FilterSet(BTreeMap<Symbol, Option<Value>>);