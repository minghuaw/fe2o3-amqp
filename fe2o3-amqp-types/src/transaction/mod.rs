//! Part 4: Transactions

mod txn_capability;
mod txn_error;

use serde_amqp::{
    primitives::{Array, Binary},
    DeserializeComposite, SerializeComposite,
};

/// 4.5.1 Coordinator
/// Target for communicating with a transaction coordinator.
///
/// <type name="coordinator" class="composite" source="list" provides="target">
/// <descriptor name="amqp:coordinator:list" code="0x00000000:0x00000030"/>
///     <field name="capabilities" type="symbol" requires="txn-capability" multiple="true"/>
/// </type>
/// The coordinator type defines a special target used for establishing a link with a transaction coordinator.
#[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "amqp:coordinator:list",
    code = "0x0000_0000:0x0000_0030",
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Coordinator {
    /// The capabilities supported at the coordinator
    ///
    /// When sent by the transaction controller (the sending endpoint), indicates the desired
    /// capabilities of the coordinator. When sent by the resource (the receiving endpoint),
    /// defined the actual capa- bilities of the coordinator. Note that it is the responsibility
    /// of the transaction controller to verify that the capabilities of the controller meet its
    /// requirements.
    ///
    /// See txn-capability.
    pub capabilities: Option<Array<TxnCapability>>,
}

impl From<Coordinator> for TargetArchetype {
    fn from(coordinator: Coordinator) -> Self {
        Self::Coordinator(coordinator)
    }
}

impl TryFrom<TargetArchetype> for Coordinator {
    type Error = TargetArchetype;

    fn try_from(value: TargetArchetype) -> Result<Self, Self::Error> {
        match value {
            TargetArchetype::Coordinator(coord) => Ok(coord),
            _ => Err(value),
        }
    }
}

/// ```rust,ignore
/// Coordinator {
///     capabilities: None,
/// }
/// ```
impl Default for Coordinator {
    fn default() -> Self {
        Self { capabilities: None }
    }
}

impl Coordinator {
    /// Creates a new coordinator
    pub fn new(capabilities: impl Into<Option<Array<TxnCapability>>>) -> Self {
        Self {
            capabilities: capabilities.into(),
        }
    }
}

/// 4.5.2 Declare
///
/// Message body for declaring a transaction id.
///
/// <type name="declare" class="composite" source="list">
///     <descriptor name="amqp:declare:list" code="0x00000000:0x00000031"/>
///     <field name="global-id" type="*" requires="global-tx-id"/>
/// </type>
///
/// The declare type defines the message body sent to the coordinator to declare a transaction.
/// The txn-id allocated for this transaction is chosen by the transaction controller and identified
/// in the declared resulting outcome.
///
#[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "amqp:declare:list",
    code = "0x0000_0000:0x0000_0031",
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Declare {
    /// global-id global transaction id
    ///
    /// Specifies that the txn-id allocated by this declare MUST be associated with the indicated global
    /// transaction. If not set, the allocated txn-id will be associated with a local transaction. This
    /// field MUST NOT be set if the coordinator does not have the distributed-transactions capability.
    /// Note that the details of distributed transactions within AMQP 1.0 will be provided in a separate
    /// specification.
    pub global_id: Option<TransactionId>,
}

impl Declare {
    /// Creates a new [`Declare`]
    pub fn new(global_id: impl Into<Option<TransactionId>>) -> Self {
        Self {
            global_id: global_id.into(),
        }
    }
}

/// 4.5.3 Discharge
///
/// Message body for discharging a transaction.
///
/// <type name="discharge" class="composite" source="list">
///     <descriptor name="amqp:discharge:list" code="0x00000000:0x00000032"/>
///     <field name="txn-id" type="*" requires="txn-id" mandatory="true"/>
///     <field name="fail" type="boolean"/>
/// </type>
///
/// The discharge type defines the message body sent to the coordinator to indicate that the txn-id
/// is no longer in use. If the transaction is not associated with a global-id, then this also
/// indicates the disposition of the local transaction.
#[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "amqp:discharge:list",
    code = "0x0000_0000:0x0000_0032",
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Discharge {
    /// Identifies the transaction to be discharged
    pub txn_id: TransactionId,

    /// Indicates the transaction has failed
    ///
    /// If set, this flag indicates that the work associated with this transaction has failed, and the
    /// controller wishes the transaction to be rolled back. If the transaction is associated with a
    /// global-id this will render the global transaction rollback-only. If the transaction is a local
    /// transaction, then this flag controls whether the transaction is committed or aborted when it is
    /// discharged. (Note that the specification for distributed transactions within AMQP 1.0 will be
    /// provided separately in Part 6 Distributed Transactions).
    pub fail: Option<bool>,
}

/// 4.5.4 Transaction ID
/// <type name="transaction-id" class="restricted" source="binary" provides="txn-id"/>
/// A transaction-id can be up to 32 octets of binary data.
pub type TransactionId = Binary;

/// 4.5.5 Declared
/// <type name="declared" class="composite" source="list" provides="delivery-state, outcome">
///     <descriptor name="amqp:declared:list" code="0x00000000:0x00000033"/>
///     <field name="txn-id" type="*" requires="txn-id" mandatory="true"/>
/// </type>
///
/// Indicates that a transaction identifier has successfully been allocated in response to a declare
/// message sent to a transaction coordinator.
#[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "amqp:declared:list",
    code = "0x0000_0000:0x0000_0033",
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Declared {
    /// The allocated transaction id
    pub txn_id: TransactionId,
}

/// 4.5.6 Transactional State
/// The state of a transactional message transfer.
/// <type name="transactional-state" class="composite" source="list" provides="delivery-state">
///     <descriptor name="amqp:transactional-state:list" code="0x00000000:0x00000034"/>
///     <field name="txn-id" type="*" mandatory="true" requires="txn-id"/>
///     <field name="outcome" type="*" requires="outcome"/>
/// </type>
/// The transactional-state type defines a delivery-state that is used to associate a delivery with
/// a transaction as well as to indicate which outcome is to be applied if the transaction commits.
#[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "amqp:transactional-state:list",
    code = "0x0000_0000:0x0000_0034",
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct TransactionalState {
    /// txn-id identifies the transaction with which the state is associated outcome provisional outcome
    pub txn_id: TransactionId,

    /// This field indicates the provisional outcome to be applied if the transaction commits.
    pub outcome: Option<crate::messaging::Outcome>,
}

// 4.5.7 Transaction Capability
pub use txn_capability::TxnCapability;

// 4.5.8 Transaction Error
pub use txn_error::TransactionError;

use crate::messaging::TargetArchetype;
