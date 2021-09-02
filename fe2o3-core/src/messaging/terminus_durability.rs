use serde::{Deserialize, Serialize};

/// 3.5.5 Terminus Durability
/// Durability policy for a terminus.
/// <type name="terminus-durability" class="restricted" source="uint">
/// </type>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TerminusDurability {
    /// <choice name="none" value="0"/>
    None,
    
    /// <choice name="configuration" value="1"/>
    Configuration,
    
    /// <choice name="unsettled-state" value="2"/>
    UnsettledState,
}

impl Default for TerminusDurability {
    fn default() -> Self {
        TerminusDurability::None
    }
}
