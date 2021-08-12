use crate::types::Symbol;

pub const DESCRIPTOR_MAGIC: &str = "DESCRIPTOR";

#[derive(Debug)]
pub struct Descriptor {
    name: Symbol,
    code: Option<u64>
}