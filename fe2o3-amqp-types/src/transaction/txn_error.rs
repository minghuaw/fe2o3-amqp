//! 4.5.8 Transaction Error

use serde::{ser, de};
use serde_amqp::primitives::Symbol;

/// 4.5.8 Transaction Error
/// 
/// Symbols used to indicate transaction errors.
/// 
/// <type name="transaction-error" class="restricted" source="symbol" provides="error-condition">
///     <choice name="unknown-id" value="amqp:transaction:unknown-id"/>
///     <choice name="transaction-rollback" value="amqp:transaction:rollback"/>
///     <choice name="transaction-timeout" value="amqp:transaction:timeout"/>
/// </type>
#[derive(Debug, Clone, PartialEq)]
pub enum TransactionError {
    /// amqp:transaction:unknown-id
    /// The specified txn-id does not exist.
    UnknownId,
    
    /// amqp:transaction:rollback
    /// The transaction was rolled back for an unspecified reason.
    Rollback,

    /// amqp:transaction:timeout
    /// The work represented by this transaction took too long.
    Timeout,
}

impl From<&TransactionError> for Symbol {
    fn from(err: &TransactionError) -> Self {
        let s = match err {
            TransactionError::UnknownId => "amqp:transaction:unknown-id",
            TransactionError::Rollback => "amqp:transaction:rollback",
            TransactionError::Timeout => "amqp:transaction:timeout",
        };

        Symbol::from(s)
    }
}

impl<'a> TryFrom<&'a str> for TransactionError {
    type Error = &'a str;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let val = match value {
            "amqp:transaction:unknown-id" => Self::UnknownId,
            "amqp:transaction:rollback" => Self::Rollback,
            "amqp:transaction:timeout" => Self::Timeout,
            _ => return Err(value)
        };

        Ok(val)
    }
}

impl TryFrom<Symbol> for TransactionError {
    type Error = Symbol;

    fn try_from(value: Symbol) -> Result<Self, Self::Error> {
        match value.as_str().try_into() {
            Ok(val) => Ok(val),
            Err(_) => Err(value)
        }
    }
}

impl ser::Serialize for TransactionError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        let val = Symbol::from(self);
        val.serialize(serializer)
    }
}

impl<'de> de::Deserialize<'de> for TransactionError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
        Symbol::deserialize(deserializer)?
            .try_into()
            .map_err(|_| de::Error::custom("Invalid symbol value for TransactionError"))
    }
}