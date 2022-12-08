//! Operations with entities

mod create;
mod delete;
mod read;
mod update;

pub use create::*;
pub use delete::*;
pub use read::*;
pub use update::*;

/// Operations that can be performed on a manageable entity
pub trait ManageableEntityOperations: Create + Read + Update + Delete {}

impl<T> ManageableEntityOperations for T where T: Create + Read + Update + Delete {}
