use fe2o3_amqp::types::Symbol;
use serde::{de, ser};

/// 3.5.6 Terminus Expiry Policy
/// <type name="terminus-expiry-policy" class="restricted" source="symbol">
///     <choice name="link-detach" value="link-detach"/>
///     <choice name="session-end" value="session-end"/>
///     <choice name="connection-close" value="connection-close"/>
///     <choice name="never" value="never"/>
/// </type>
///
/// TODO: manually implement serialize and deserialize
#[derive(Debug)]
pub enum TerminusExpiryPolicy {
    LinkDetach,
    SessionEnd,
    ConnectionClose,
    Never,
}

impl From<&TerminusExpiryPolicy> for Symbol {
    fn from(value: &TerminusExpiryPolicy) -> Self {
        let val = match value {
            &TerminusExpiryPolicy::LinkDetach => "link-detach",
            &TerminusExpiryPolicy::SessionEnd => "session-end",
            &TerminusExpiryPolicy::ConnectionClose => "connection-close",
            &TerminusExpiryPolicy::Never => "never",
        };
        Symbol::from(val)
    }
}

impl ser::Serialize for TerminusExpiryPolicy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Symbol::from(self).serialize(serializer)
    }
}

enum Field {
    LinkDetach,
    SessionEnd,
    ConnectionClose,
    Never,
}

struct FieldVisitor { }

impl<'de> de::Visitor<'de> for FieldVisitor {
    type Value = Field;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("field identifier")
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
            E: de::Error, {
        self.visit_str(&v)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
            E: de::Error, {
        let val = match v {
            "link-detach" => Field::LinkDetach,
            "session-end" => Field::SessionEnd,
            "connection-close" => Field::ConnectionClose,
            "never" => Field::Never,
            _ => return Err(de::Error::custom("Invalid symbol value for TerminusExpiryPolicy")),
        };
        Ok(val)
    }
}

impl<'de> de::Deserialize<'de> for Field {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
            D: serde::Deserializer<'de> {
        deserializer.deserialize_identifier(FieldVisitor { })
    }
}

struct Visitor { }

impl<'de> de::Visitor<'de> for Visitor {
    type Value = TerminusExpiryPolicy;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("enum TerminusExpiryPolicy")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
            A: de::EnumAccess<'de>, {
        let (val, _) = data.variant()?;
        let val = match val {
            Field::LinkDetach => TerminusExpiryPolicy::LinkDetach,
            Field::SessionEnd => TerminusExpiryPolicy::SessionEnd,
            Field::ConnectionClose => TerminusExpiryPolicy::ConnectionClose,
            Field::Never => TerminusExpiryPolicy::Never
        };
        Ok(val)
    }
}

impl<'de> de::Deserialize<'de> for TerminusExpiryPolicy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const VARIANTS: &'static [&'static str] = &[
            "link-detach",
            "session-end",
            "connection-close",
            "never",
        ];
        deserializer.deserialize_enum("terminus-expiry-policy", VARIANTS, Visitor { })
    }
}
