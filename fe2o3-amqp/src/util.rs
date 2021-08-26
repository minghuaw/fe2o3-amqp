pub(crate) enum NewType {
    None,
    Array,
    Dec32,
    Dec64,
    Dec128,
    Symbol,
    Timestamp,
    Uuid,
}

impl Default for NewType {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone)]
pub enum IsArrayElement {
    False,
    FirstElement,
    OtherElement,
}

// pub const AMQP_ERROR: &str = "AMQP_ERROR";
// pub const CONNECTION_ERROR: &str = "CONNECTION_ERROR";
// pub const SESSION_ERROR: &str = "SESSION_ERROR";
// pub const LINK_ERROR: &str = "LINK_ERROR";

#[derive(Debug, Clone)]
pub enum EnumType {
    None,
    Descriptor,
    Value,
    // AmqpError,
    // ConnectionError,
    // SessionError,
    // LinkError,
}

impl Default for EnumType {
    fn default() -> Self {
        Self::None
    }
}
