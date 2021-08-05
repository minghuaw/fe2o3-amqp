use super::Descriptor;

pub struct Described<'a, T> {
    pub descriptor: Descriptor,
    pub value: &'a T
}