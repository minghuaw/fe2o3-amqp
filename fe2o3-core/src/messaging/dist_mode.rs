use serde::{de, ser};

/// 3.5.7 Standard Distribution Mode
/// Link distribution policy.
/// <type name="std-dist-mode" class="restricted" source="symbol" provides="distribution-mode">
///     <choice name="move" value="move"/>
///     <choice name="copy" value="copy"/>
/// </type>
///
/// TODO: manually impl serialize and deserialize
#[derive(Debug)]
pub enum DistributionMode {
    Move,
    Copy,
}

impl ser::Serialize for DistributionMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        todo!()
    }
}

impl<'de> de::Deserialize<'de> for DistributionMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        todo!()
    }
}
