pub const UUID: &str = "UUID";

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Uuid([u8; 16]);

// TODO: optional conversion to external type uuid::Uuid;