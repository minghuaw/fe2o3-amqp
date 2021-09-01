use serde::{ser, de};

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

impl ser::Serialize for TerminusExpiryPolicy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
            S: serde::Serializer {
        todo!()
    }
}

impl<'de> de::Deserialize<'de> for TerminusExpiryPolicy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
            D: serde::Deserializer<'de> {
        todo!()
    }
}