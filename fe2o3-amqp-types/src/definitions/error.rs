use serde_amqp::macros::{DeserializeComposite, SerializeComposite};

use super::{ErrorCondition, Fields};

/// <type name="error" class="composite" source="list">
/// <descriptor name="amqp:error:list" code="0x00000000:0x0000001d"/>
/// </type>
#[derive(Debug, Clone, PartialEq, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "amqp:error:list",
    code = 0x0000_0000_0000_001d,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Error {
    /// <field name="condition" type="symbol" requires="error-condition" mandatory="true"/>
    condition: ErrorCondition,
    
    /// <field name="description" type="string"/>
    description: Option<String>,
    
    /// <field name="info" type="fields"/>
    info: Option<Fields>,
}

impl Error {
    pub fn new(
        condition: impl Into<ErrorCondition>,
        description: Option<String>,
        info: Option<Fields>,
    ) -> Self {
        Self {
            condition: condition.into(),
            description: description.map(Into::into),
            info: info.map(Into::into),
        }
    }
}

impl<T> From<T> for Error
where
    T: Into<ErrorCondition>,
{
    fn from(condition: T) -> Self {
        Self {
            condition: condition.into(),
            description: None,
            info: None,
        }
    }
}

#[cfg(test)]
mod tests {

    use serde_amqp::{from_slice, to_vec};

    use crate::definitions::AmqpError;

    use super::Error;

    #[test]
    fn test_serde_error() {
        let expected = Error::new(AmqpError::DecodeError, None, None);
        let serialized = to_vec(&expected).unwrap();
        let deserialized: Error = from_slice(&serialized).unwrap();
        assert_eq!(expected, deserialized)
    }
}
