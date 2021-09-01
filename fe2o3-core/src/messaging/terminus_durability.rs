use serde::{Serialize, Deserialize};

/// 3.5.5 Terminus Durability
/// Durability policy for a terminus.
/// <type name="terminus-durability" class="restricted" source="uint">
///     <choice name="none" value="0"/>
///     <choice name="configuration" value="1"/>
///     <choice name="unsettled-state" value="2"/>
/// </type>
#[derive(Debug, Serialize, Deserialize)]
pub enum TerminusDurability {
    None,
    Configuration,
    UnsettledState
}