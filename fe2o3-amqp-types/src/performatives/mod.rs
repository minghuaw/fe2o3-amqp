//! Performatives defined in AMQP 1.0 specification Part 2.7

mod attach;
mod begin;
mod close;
mod detach;
mod disposition;
mod end;
mod flow;
mod open;
mod transfer;

pub use attach::*;
pub use begin::*;
pub use close::*;
pub use detach::*;
pub use disposition::*;
pub use end::*;
pub use flow::*;
pub use open::*;
pub use transfer::*;

/// AMQP 1.0 Performatives
#[derive(Debug, Clone)]
pub enum Performative {
    /// Open
    Open(Open),

    /// Begin
    Begin(Begin),

    /// Attach
    Attach(Attach),

    /// Flow
    Flow(Flow),

    /// Transfer
    Transfer(Transfer),

    /// Disposition
    Disposition(Disposition),

    /// Detach
    Detach(Detach),

    /// End
    End(End),

    /// Close
    Close(Close),
}

mod performative_impl {
    use serde::{
        de::{self, VariantAccess},
        ser,
    };

    use super::Performative;

    impl ser::Serialize for Performative {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            match self {
                Performative::Open(value) => value.serialize(serializer),
                Performative::Begin(value) => value.serialize(serializer),
                Performative::Attach(value) => value.serialize(serializer),
                Performative::Flow(value) => value.serialize(serializer),
                Performative::Transfer(value) => value.serialize(serializer),
                Performative::Disposition(value) => value.serialize(serializer),
                Performative::Detach(value) => value.serialize(serializer),
                Performative::End(value) => value.serialize(serializer),
                Performative::Close(value) => value.serialize(serializer),
            }
        }
    }

    enum Field {
        Open,
        Begin,
        Attach,
        Flow,
        Transfer,
        Disposition,
        Detach,
        End,
        Close,
    }

    struct FieldVisitor {}

    impl<'de> de::Visitor<'de> for FieldVisitor {
        type Value = Field;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("variant identifier")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let val = match v {
                "amqp:open:list" => Field::Open,
                "amqp:begin:list" => Field::Begin,
                "amqp:attach:list" => Field::Attach,
                "amqp:flow:list" => Field::Flow,
                "amqp:transfer:list" => Field::Transfer,
                "amqp:disposition:list" => Field::Disposition,
                "amqp:detach:list" => Field::Detach,
                "amqp:end:list" => Field::End,
                "amqp:close:list" => Field::Close,
                _ => return Err(de::Error::custom("Wrong symbol value for descriptor")),
            };

            Ok(val)
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let val = match v {
                0x0000_0000_0000_0010 => Field::Open,
                0x0000_0000_0000_0011 => Field::Begin,
                0x0000_0000_0000_0012 => Field::Attach,
                0x0000_0000_0000_0013 => Field::Flow,
                0x0000_0000_0000_0014 => Field::Transfer,
                0x0000_0000_0000_0015 => Field::Disposition,
                0x0000_0000_0000_0016 => Field::Detach,
                0x0000_0000_0000_0017 => Field::End,
                0x0000_0000_0000_0018 => Field::Close,
                _ => {
                    return Err(de::Error::custom(format!(
                        "Wrong code value for descriptor, found {:#x?}",
                        v
                    )))
                }
            };
            Ok(val)
        }
    }

    impl<'de> de::Deserialize<'de> for Field {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_identifier(FieldVisitor {})
        }
    }

    struct Visitor {}

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = Performative;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("enum DeliveryState")
        }

        fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
        where
            A: de::EnumAccess<'de>,
        {
            let (val, variant) = data.variant()?;

            match val {
                Field::Open => {
                    let value = variant.newtype_variant()?;
                    Ok(Performative::Open(value))
                }
                Field::Begin => {
                    let value = variant.newtype_variant()?;
                    Ok(Performative::Begin(value))
                }
                Field::Attach => {
                    let value = variant.newtype_variant()?;
                    Ok(Performative::Attach(value))
                }
                Field::Flow => {
                    let value = variant.newtype_variant()?;
                    Ok(Performative::Flow(value))
                }
                Field::Transfer => {
                    let value = variant.newtype_variant()?;
                    Ok(Performative::Transfer(value))
                }
                Field::Disposition => {
                    let value = variant.newtype_variant()?;
                    Ok(Performative::Disposition(value))
                }
                Field::Detach => {
                    let value = variant.newtype_variant()?;
                    Ok(Performative::Detach(value))
                }
                Field::End => {
                    let value = variant.newtype_variant()?;
                    Ok(Performative::End(value))
                }
                Field::Close => {
                    let value = variant.newtype_variant()?;
                    Ok(Performative::Close(value))
                }
            }
        }
    }

    impl<'de> de::Deserialize<'de> for Performative {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            const VARIANTS: &[&str] = &[
                "amqp:open:list",
                "amqp:begin:list",
                "amqp:attach:list",
                "amqp:flow:list",
                "amqp:transfer:list",
                "amqp:disposition:list",
                "amqp:detach:list",
                "amqp:end:list",
                "amqp:close:list",
            ];
            deserializer.deserialize_enum("Performative", VARIANTS, Visitor {})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Performative;

    use super::End;
    use serde_amqp::de::from_slice;
    use serde_amqp::ser::to_vec;

    #[test]
    fn test_untagged_serde() {
        let end = Performative::End(End { error: None });
        let buf = to_vec(&end).unwrap();
        let end2: Result<Performative, _> = from_slice(&buf);
        println!("{:x?}", buf);
        println!("{:?}", end2);
    }

    #[test]
    fn test_size_of_variants() {
        use super::*;
        use std::mem::size_of;

        println!("Performative {:?}", size_of::<Performative>());
        println!("Open {:?}", size_of::<Open>());
        println!("Begin {:?}", size_of::<Begin>());
        println!("Attach {:?}", size_of::<Attach>());
        println!("Flow {:?}", size_of::<Flow>());
        println!("Transfer {:?}", size_of::<Transfer>());
        println!("Disposition {:?}", size_of::<Disposition>());
        println!("Detach {:?}", size_of::<Detach>());
        println!("End {:?}", size_of::<End>());
        println!("Close {:?}", size_of::<Close>());
    }
}
