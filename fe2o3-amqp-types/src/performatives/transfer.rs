use serde_amqp::{
    macros::{DeserializeComposite, SerializeComposite},
    primitives::Boolean,
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
    ///
    /// Specifies the link on which the message is transferred.
    pub handle: Handle,

    /// <field name="delivery-id" type="delivery-number"/>
    ///
    /// The delivery-id MUST be supplied on the first transfer of a
    /// multi-transfer delivery. On continuation transfers the delivery-id
    /// MAY be omitted. It is an error if the delivery-id on a continuation
    /// transfer differs from the delivery-id on the first transfer of a delivery.
    pub delivery_id: Option<DeliveryNumber>,

    /// <field name="delivery-tag" type="delivery-tag"/>
    ///
    /// Uniquely identifies the delivery attempt for a given message on this link.
    /// This field MUST be specified for the first transfer of a multi-transfer
    /// message and can only be omitted for continuation transfers. It is an error
    /// if the delivery-tag on a continuation transfer differs from the
    /// delivery-tag on the first transfer of a delivery.
    pub delivery_tag: Option<DeliveryTag>,

    /// <field name="message-format" type="message-format"/>
    ///
    /// This field MUST be specified for the first transfer of a multi-transfer
    /// message and can only be omitted for continuation transfers. It is an
    /// error if the message-format on a continuation transfer differs from the
    /// message-format on the first transfer of a delivery.
    pub message_format: Option<MessageFormat>,

    /// <field name="settled" type="boolean"/>
    ///
    /// If not set on the first (or only) transfer for a (multi-transfer)
    /// delivery, then the settled flag MUST be interpreted as being false. For
    /// subsequent transfers in a multi-transfer delivery if the settled flag
    /// is left unset then it MUST be interpreted as true if and only if the
    /// value of the settled flag on any of the preceding transfers was true;
    /// if no preceding transfer was sent with settled being true then the
    /// value when unset MUST be taken as false.
    ///
    /// If the negotiated value for snd-settle-mode at attachment is settled,
    /// then this field MUST be true on at least one transfer frame for a
    /// delivery (i.e., the delivery MUST be settled at the sender at the point
    /// the delivery has been completely transferred).
    ///
    /// If the negotiated value for snd-settle-mode at attachment is unsettled,
    /// then this field MUST be false (or unset) on every transfer frame for a
    /// delivery (unless the delivery is aborted).
    pub settled: Option<Boolean>,

    /// <field name="more" type="boolean" default="false"/>
    ///
    /// Note that if both the more and aborted fields are set to true, the
    /// aborted flag takes precedence. That is, a receiver SHOULD ignore the
    /// value of the more field if the transfer is marked as aborted. A sender
    /// SHOULD NOT set the more flag to true if it also sets the aborted flag
    /// to true.
    #[amqp_contract(default)]
    pub more: Boolean,

    /// <field name="rcv-settle-mode" type="receiver-settle-mode"/>
    ///
    /// If first, this indicates that the receiver MUST settle the delivery
    /// once it has arrived without waiting for the sender to settle first.
    ///
    /// If second, this indicates that the receiver MUST NOT settle until
    /// sending its disposition to the sender and receiving a settled
    /// disposition from the sender.
    ///
    /// If not set, this value is defaulted to the value negotiated on link
    /// attach.
    ///
    /// If the negotiated link value is first, then it is illegal to set this
    /// field to second.
    ///
    /// If the message is being sent settled by the sender, the value of this
    /// field is ignored.
    ///
    /// The (implicit or explicit) value of this field does not form part of
    /// the transfer state, and is not retained if a link is suspended and
    /// subsequently resumed.
    pub rcv_settle_mode: Option<ReceiverSettleMode>,

    /// <field name="state" type="*" requires="delivery-state"/>
    ///
    /// When set this informs the receiver of the state of the delivery at the
    /// sender. This is particularly useful when transfers of unsettled
    /// deliveries are resumed after resuming a link. Setting the state on the
    /// transfer can be thought of as being equivalent to sending a disposition
    /// immediately before the transfer performative, i.e., it is the state of
    /// the delivery (not the transfer) that existed at the point the frame was
    /// sent.
    ///
    /// Note that if the transfer performative (or an earlier disposition
    /// performative referring to the delivery) indicates that the delivery has
    /// attained a terminal state, then no future transfer or disposition sent
    /// by the sender can alter that terminal state.
    pub state: Option<DeliveryState>,

    /// <field name="resume" type="boolean" default="false"/>
    ///
    /// If true, the resume flag indicates that the transfer is being used to
    /// reassociate an unsettled delivery from a dissociated link endpoint. See
    /// subsection 2.6.13 for more details.
    ///
    /// The receiver MUST ignore resumed deliveries that are not in its local
    /// unsettled map. The sender MUST NOT send resumed transfers for
    /// deliveries not in its local unsettled map
    ///
    /// If a resumed delivery spans more than one transfer performative, then
    /// the resume flag MUST be set to true on the first transfer of the resumed
    /// delivery. For subsequent transfers for the same delivery the resume flag
    /// MAY be set to true, or MAY be omitted.
    ///
    /// In the case where the exchange of unsettled maps makes clear that all
    /// message data has been successfully transferred to the receiver, and that
    /// only the final state (and potentially settlement) at the sender needs to
    /// be conveyed, then a resumed delivery MAY carry no payload and instead
    /// act solely as a vehicle for carrying the terminal state of the delivery
    /// at the sender.
    #[amqp_contract(default)]
    pub resume: Boolean,

    /// <field name="aborted" type="boolean" default="false"/>
    ///
    /// Aborted messages SHOULD be discarded by the recipient (any payload
    /// within the frame carrying the performative MUST be ignored). An aborted
    /// message is implicitly settled
    #[amqp_contract(default)]
    pub aborted: Boolean,

    /// <field name="batchable" type="boolean" default="false"/>
    ///
    /// If true, then the issuer is hinting that there is no need for the peer
    /// to urgently communicate updated delivery state. This hint MAY be used to
    /// artificially increase the amount of batching an implementation uses when
    /// communicating delivery states, and thereby save bandwidth.
    ///
    /// If the message being delivered is too large to fit within a single
    /// frame, then the setting of batchable to true on any of the transfer
    /// performatives for the delivery is equivalent to setting batchable to
    /// true for all the transfer performatives for the delivery.
    ///
    /// The batchable value does not form part of the transfer state, and is not
    /// retained if a link is sus- pended and subsequently resumed.
    #[amqp_contract(default)]
    pub batchable: Boolean,
}
