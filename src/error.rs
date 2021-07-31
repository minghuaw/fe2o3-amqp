use std::fmt::Display;
use serde::{ser, de};

// pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    Message(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unimplemented!()
    }
}

impl std::error::Error for Error { }

impl ser::Error for Error {
    fn custom<T>(msg:T) ->Self
    where
        T: std::fmt::Display 
    {
        unimplemented!()
    }
}

impl de::Error for Error {
    fn custom<T>(msg:T) ->Self
    where
        T:Display 
    {
        unimplemented!()    
    }
}