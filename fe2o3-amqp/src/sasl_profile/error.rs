/// TODO:
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Not implemented {0:?}")]
    NotImplemented(Option<String>),
}
