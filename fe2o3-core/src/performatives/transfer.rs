use fe2o3_amqp::{
    macros::{DeserializeComposite, SerializeComposite},
    types::Boolean,
};

use crate::{
    definitions::{DeliveryNumber, DeliveryTag, Handle, MessageFormat, ReceiverSettleMode},
    messaging::DeliveryState,
};

/// 2.7.5 Transfer
/// Transfer a message.
/// <type name="transfer" class="composite" source="list" provides="frame">
///     <descriptor name="amqp:transfer:list" code="0x00000000:0x00000014"/>
/// </type>
#[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
// #[serde(rename_all = "kebab-case")]
#[amqp_contract(
    name = "amqp:transfer:list",
    code = 0x0000_0000_0000_0014,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Transfer {
    /// <field name="handle" type="handle" mandatory="true"/>
    pub handle: Handle,
    
    /// <field name="delivery-id" type="delivery-number"/>
    pub delivery_id: Option<DeliveryNumber>,
    
    /// <field name="delivery-tag" type="delivery-tag"/>
    pub delivery_tag: Option<DeliveryTag>,
    
    /// <field name="message-format" type="message-format"/>
    pub message_format: Option<MessageFormat>,
    
    /// <field name="settled" type="boolean"/>
    pub settled: Option<Boolean>,
    
    /// <field name="more" type="boolean" default="false"/>
    #[amqp_contract(default)]
    pub more: Boolean,
    
    /// <field name="rcv-settle-mode" type="receiver-settle-mode"/>
    pub rcv_settle_mode: Option<ReceiverSettleMode>,
    
    /// <field name="state" type="*" requires="delivery-state"/>
    pub state: Option<DeliveryState>,
    
    /// <field name="resume" type="boolean" default="false"/>
    #[amqp_contract(default)]
    pub resume: Boolean,
    
    /// <field name="aborted" type="boolean" default="false"/>
    #[amqp_contract(default)]
    pub aborted: Boolean,
    
    /// <field name="batchable" type="boolean" default="false"/>
    #[amqp_contract(default)]
    pub batchable: Boolean,
}
