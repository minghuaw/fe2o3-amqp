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

impl DeclareError {
    pub(crate) fn new(controller: Controller<Undeclared>, error: link::Error) -> Self {
        Self {
            controller,
            error
        }
    }
}