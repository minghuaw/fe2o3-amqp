use serde::{Deserialize, Serialize};

/// 3.5.5 Terminus Durability
/// Durability policy for a terminus.
/// <type name="terminus-durability" class="restricted" source="uint">
/// </type>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TerminusDurability {
    /// <choice name="none" value="0"/>
    #[default]
    None,

    /// <choice name="configuration" value="1"/>
    Configuration,

    /// <choice name="unsettled-state" value="2"/>
    UnsettledState,
}

// TODO: test serialization and deserialization
