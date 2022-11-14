use std::borrow::Cow;

use fe2o3_amqp_types::messaging::ApplicationProperties;

use crate::constants::ENTITY_TYPE;

pub(crate) struct GetRequest<'a> {
    pub entity_type: Option<Cow<'a, str>>,

    /// Entity type
    pub manageable_entity_type: Cow<'a, str>,

    /// locales
    pub locales: Option<Cow<'a, str>>,
}

impl<'a> GetRequest<'a> {
    pub fn new(
        entity_type: impl Into<Option<Cow<'a, str>>>,
        manageable_entity_type: impl Into<Cow<'a, str>>,
        locales: Option<impl Into<Cow<'a, str>>>,
    ) -> Self {
        Self {
            entity_type: entity_type.into(),
            manageable_entity_type: manageable_entity_type.into(),
            locales: locales.map(|x| x.into()),
        }
    }

    pub(crate) fn manageable_entity_type(&mut self) -> Option<String> {
        Some(self.manageable_entity_type.to_string())
    }

    pub(crate) fn locales(&mut self) -> Option<String> {
        self.locales.as_ref().map(|x| x.to_string())
    }

    pub(crate) fn encode_application_properties(&mut self) -> Option<ApplicationProperties> {
        self.entity_type.as_ref().map(|entity_type| {
            ApplicationProperties::builder()
                .insert(ENTITY_TYPE, entity_type.to_string())
                .build()
        })
    }
}
