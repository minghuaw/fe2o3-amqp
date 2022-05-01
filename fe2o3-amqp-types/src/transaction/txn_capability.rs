//! 4.5.7 Transaction Capability

use serde::{de, ser};
use serde_amqp::primitives::Symbol;

/// 4.5.7 Transaction Capability
///
/// Symbols indicating (desired/available) capabilities of a transaction coordinator.
///
/// <type name="txn-capability" class="restricted" source="symbol" provides="txn-capability">
///     <choice name="local-transactions" value="amqp:local-transactions"/>
///     <choice name="distributed-transactions" value="amqp:distributed-transactions"/>
///     <choice name="promotable-transactions" value="amqp:promotable-transactions"/>
///     <choice name="multi-txns-per-ssn" value="amqp:multi-txns-per-ssn"/>
///     <choice name="multi-ssns-per-txn" value="amqp:multi-ssns-per-txn"/>
/// </type>
#[derive(Debug, Clone)]
pub enum TxnCapability {
    /// amqp:local-transactions
    /// Support local transactions.
    LocalTransactions,

    /// amqp:distributed-transactions
    /// Support AMQP Distributed Transactions.
    DistributedTransactions,

    /// amqp:promotable-transactions
    /// Support AMQP Promotable Transactions.
    PromotableTransactions,

    /// amqp:multi-txns-per-ssn
    /// Support multiple active transactions on a single session.
    MultiTxnsPerSsn,

    /// amqp:multi-ssns-per-txn
    /// Support transactions whose txn-id is used across sessions on one connection.
    MultiSsnsPerTxn,
}

impl From<&TxnCapability> for Symbol {
    fn from(value: &TxnCapability) -> Self {
        let s = match value {
            TxnCapability::LocalTransactions => "amqp:local-transactions",
            TxnCapability::DistributedTransactions => "amqp:distributed-transactions",
            TxnCapability::PromotableTransactions => "amqp:promotable-transactions",
            TxnCapability::MultiTxnsPerSsn => "amqp:multi-txns-per-ssn",
            TxnCapability::MultiSsnsPerTxn => "amqp:multi-ssns-per-txn",
        };

        Symbol::from(s)
    }
}

impl TryFrom<Symbol> for TxnCapability {
    type Error = Symbol;

    fn try_from(value: Symbol) -> Result<Self, Self::Error> {
        match value.as_str().try_into() {
            Ok(val) => Ok(val),
            Err(_) => Err(value),
        }
    }
}

impl<'a> TryFrom<&'a str> for TxnCapability {
    type Error = &'a str;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let val = match value {
            "amqp:local-transactions" => Self::LocalTransactions,
            "amqp:distributed-transactions" => Self::DistributedTransactions,
            "amqp:promotable-transactions" => Self::PromotableTransactions,
            "amqp:multi-txns-per-ssn" => Self::MultiTxnsPerSsn,
            "amqp:multi-ssns-per-txn" => Self::MultiSsnsPerTxn,
            _ => return Err(value),
        };

        Ok(val)
    }
}

impl ser::Serialize for TxnCapability {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let val = Symbol::from(self);
        val.serialize(serializer)
    }
}

impl<'de> de::Deserialize<'de> for TxnCapability {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Symbol::deserialize(deserializer)?
            .try_into()
            .map_err(|_| de::Error::custom("Invalid symbol value for TxnCapability"))
    }
}
