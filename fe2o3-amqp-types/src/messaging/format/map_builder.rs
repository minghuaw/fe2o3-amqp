//! AnnotationBuilder for types that are simply a wrapper around Annotation

use std::{collections::BTreeMap, marker::PhantomData};

use serde_amqp::{primitives::Symbol, Value};

use crate::primitives::SimpleValue;

use super::{ApplicationProperties, DeliveryAnnotations, Footer, MessageAnnotations};

/// Builder for types that are simply a wrapper around map
/// ([`DeliveryAnnotations`], [`MessageAnnotations`], [`Footer`], [`ApplicationProperties`])
///
/// This simply provides a convenient way of inserting entries into the map
#[derive(Debug)]
pub struct MapBuilder<K, V, T>
where
    K: Ord,
{
    map: BTreeMap<K, V>,
    marker: PhantomData<T>,
}

impl<K: Ord, V, T> Default for MapBuilder<K, V, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V, T> MapBuilder<K, V, T>
where
    K: Ord,
{
    /// Creates a new builder for annotation types
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
            marker: PhantomData,
        }
    }

    /// A convenience method to insert an entry into the annotation map
    pub fn insert(mut self, key: impl Into<K>, value: impl Into<V>) -> Self {
        self.map.insert(key.into(), value.into());
        self
    }
}

impl MapBuilder<Symbol, Value, DeliveryAnnotations> {
    /// Build [`DeliveryAnnotations`]
    pub fn build(self) -> DeliveryAnnotations {
        DeliveryAnnotations(self.map)
    }
}

impl MapBuilder<Symbol, Value, MessageAnnotations> {
    /// Build [`MessageAnnotations`]
    pub fn build(self) -> MessageAnnotations {
        MessageAnnotations(self.map)
    }
}

impl MapBuilder<Symbol, Value, Footer> {
    /// Build [`Footer`]
    pub fn build(self) -> Footer {
        Footer(self.map)
    }
}

impl MapBuilder<String, SimpleValue, ApplicationProperties> {
    /// Build [`ApplicationProperties`]
    pub fn build(self) -> ApplicationProperties {
        ApplicationProperties(self.map)
    }
}

#[cfg(test)]
mod tests {
    use crate::messaging::{ApplicationProperties, MessageAnnotations};

    #[test]
    fn test_message_annotation_builder() {
        let message_annotation = MessageAnnotations::builder()
            .insert("key", "value")
            .insert("key2", 1)
            .build();
        println!("{:?}", message_annotation);
    }

    #[test]
    fn test_application_properties_builder() {
        let application_props = ApplicationProperties::builder()
            .insert("key", "value")
            .insert("key2", 1i32)
            .build();
        println!("{:?}", application_props);
    }
}
