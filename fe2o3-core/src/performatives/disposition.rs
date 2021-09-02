use fe2o3_amqp::{
    macros::{DeserializeComposite, SerializeComposite},
    types::Boolean,
};

use crate::{
    definitions::{DeliveryNumber, Role},
    messaging::DeliveryState,
};

/// 2.7.6 Disposition
/// Inform remote peer of delivery state changes.
/// <type name="disposition" class="composite" source="list" provides="frame">
///     <descriptor name="amqp:disposition:list" code="0x00000000:0x00000015"/>
/// </type>
#[derive(Debug, DeserializeComposite, SerializeComposite)]
// #[serde(rename_all = "kebab-case")]
#[amqp_contract(
    name = "amqp:disposition:list",
    code = 0x0000_0000_0000_0015,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Disposition {
    /// <field name="role" type="role" mandatory="true"/>
    pub role: Role,
    
    /// <field name="first" type="delivery-number" mandatory="true"/>
    pub first: DeliveryNumber,
    
    /// <field name="last" type="delivery-number"/>
    pub last: Option<DeliveryNumber>,
    
    /// <field name="settled" type="boolean" default="false"/>
    #[amqp_contract(default)]
    pub settled: Boolean,
    
    /// <field name="state" type="*" requires="delivery-state"/>
    pub state: Option<DeliveryState>,
    
    /// <field name="batchable" type="boolean" default="false"/>
    #[amqp_contract(default)]
    pub batchable: Boolean,
}
