//! Controls for Connection, Session, and Link

use fe2o3_amqp_types::{definitions::Error};

pub enum ConnectionControl {
    Open,
    Begin,
    Close(Option<Error>),
}

pub enum SessionControl {

}

pub enum LinkControl {
    
}