mod create;
mod delete;
mod read;
mod update;

pub use create::*;
pub use delete::*;
pub use read::*;
pub use update::*;

pub trait ManageableEntityOperations: Create + Read + Update + Delete {}

impl<T> ManageableEntityOperations for T where T: Create + Read + Update + Delete {}
