//! Defines status code

use std::num::NonZeroU16;

use fe2o3_amqp_types::primitives::SimpleValue;

/// HTTP status code
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct StatusCode(pub NonZeroU16);

impl TryFrom<SimpleValue> for StatusCode {
    type Error = SimpleValue;

    fn try_from(value: SimpleValue) -> Result<Self, Self::Error> {
        let code = match value {
            SimpleValue::Ushort(val) => {
                let val = val;
                NonZeroU16::new(val)
            }
            SimpleValue::Uint(val) => {
                let val = val as u16;
                NonZeroU16::new(val)
            }
            SimpleValue::Ulong(val) => {
                let val = val as u16;
                NonZeroU16::new(val)
            }
            SimpleValue::Short(val) => {
                let val = val as u16;
                NonZeroU16::new(val)
            }
            SimpleValue::Int(val) => {
                let val = val as u16;
                NonZeroU16::new(val)
            }
            SimpleValue::Long(val) => {
                let val = val as u16;
                NonZeroU16::new(val)
            }
            _ => return Err(value),
        }
        .ok_or(value)?;

        Ok(StatusCode(code))
    }
}

impl<'a> TryFrom<&'a SimpleValue> for StatusCode {
    type Error = &'a SimpleValue;

    fn try_from(value: &'a SimpleValue) -> Result<Self, Self::Error> {
        let code = match value {
            SimpleValue::Ushort(val) => {
                let val = *val;
                NonZeroU16::new(val)
            }
            SimpleValue::Uint(val) => {
                let val = *val as u16;
                NonZeroU16::new(val)
            }
            SimpleValue::Ulong(val) => {
                let val = *val as u16;
                NonZeroU16::new(val)
            }
            SimpleValue::Short(val) => {
                let val = *val as u16;
                NonZeroU16::new(val)
            }
            SimpleValue::Int(val) => {
                let val = *val as u16;
                NonZeroU16::new(val)
            }
            SimpleValue::Long(val) => {
                let val = *val as u16;
                NonZeroU16::new(val)
            }
            _ => return Err(value),
        }
        .ok_or(value)?;

        Ok(StatusCode(code))
    }
}
