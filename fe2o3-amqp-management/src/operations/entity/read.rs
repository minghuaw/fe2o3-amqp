use std::collections::BTreeMap;

use crate::{Extractor, IntoResponse, error::Result};

pub trait Read {
    type Req: Extractor;
    type Res: IntoResponse;

    fn read(&mut self, arg: Self::Req) -> Result<Self::Res>;
}

pub struct ReadRequestProperties {
    /// The name of the Manageable Entity to be managed. This is case-sensitive.
    pub name: String,

    /// The identity of the Manageable Entity to be managed. This is case-sensitive.
    pub identity: String,
}

pub struct ReadResponse {
    entity_attributes: BTreeMap<String, String>,
}

impl ReadResponse {
    const STATUS_CODE: u16 = 200;
}