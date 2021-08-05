use super::Symbol;

#[derive(Debug)]
pub enum Descriptor {
    Name(Symbol),
    Code(u64),
}