use std::borrow::Cow;

use fe2o3_amqp_types::{
    messaging::{message, ApplicationProperties, Message},
    primitives::SimpleValue,
};

use crate::constants::{ENTITY_TYPE, LOCALES, TYPE};

pub(crate) struct GetRequest<'a> {
    pub entity_type: Option<Cow<'a, str>>,

    /// Entity type
    pub r#type: Cow<'a, str>,

    /// locales
    pub locales: Option<Cow<'a, str>>,
}

impl<'a> GetRequest<'a> {
    pub fn new(
        entity_type: impl Into<Option<Cow<'a, str>>>,
        r#type: impl Into<Cow<'a, str>>,
        locales: Option<impl Into<Cow<'a, str>>>,
    ) -> Self {
        Self {
            entity_type: entity_type.into(),
            r#type: r#type.into(),
            locales: locales.map(|x| x.into()),
        }
    }

    pub(crate) fn into_application_properties(self) -> ApplicationProperties {
        let mut builder = ApplicationProperties::builder()
            .insert(TYPE, SimpleValue::String(self.r#type.into()))
            .insert(
                LOCALES,
                self.locales
                    .map(|s| SimpleValue::String(s.into()))
                    .unwrap_or(SimpleValue::Null),
            );
        if let Some(entity_type) = self.entity_type {
            builder = builder.insert(ENTITY_TYPE, SimpleValue::String(entity_type.into()));
        }
        builder.build()
    }
}
