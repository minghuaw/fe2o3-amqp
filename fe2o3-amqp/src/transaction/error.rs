use crate::link;

use super::{Controller, Undeclared};

/// An error associated declaring a transaction
#[derive(Debug)]
pub struct DeclareError {
    /// The controller used for declaration
    pub controller: Controller<Undeclared>,

    /// Error associated with the declaration
    pub error: link::Error
}