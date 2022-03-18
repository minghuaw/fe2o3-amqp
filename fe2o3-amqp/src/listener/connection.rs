//! Connection Listener

use std::time::Duration;

use fe2o3_amqp_types::{performatives::MaxFrameSize, definitions::{Milliseconds, MIN_MAX_FRAME_SIZE}};
use futures_util::sink::With;
use tokio::io::AsyncReadExt;

use crate::{connection::{OpenError, WithContainerId}, transport::{protocol_header::{ProtocolHeader, ProtocolId}, Transport}, frames::amqp};

use super::Listener;

type ConnectionBuilder<'a, Tls> = crate::connection::Builder<'a, WithContainerId, Tls>;

/// Listener for incoming connections
pub struct ConnectionListener<L: Listener> {
    idle_time_out: Option<Milliseconds>,
    listener: L
}

impl<L: Listener> std::fmt::Debug for ConnectionListener<L> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionListener")
            // .field("builder", &self.builder)
            // .field("listener", &self.listener)
            .finish()
    }
}

impl<L> ConnectionListener<L> 
where
    L: Listener
{
    // /// Bind to a stream listener
    // pub fn bind(listener: L) -> Self {
    //     Self {
    //         listener
    //     }
    // }

    /// Accepts incoming connection
    pub async fn accept(&mut self) -> Result<(), OpenError> {
        let mut stream = self.listener.accept().await?;

        // Read protocol header
        let mut buf = [0u8; 8];
        stream.read_exact(&mut buf).await?;
        
        let header = match ProtocolHeader::try_from(buf) {
            Ok(header) => header,
            Err(buf) => {
                // Write protocol header and then disconnect the stream
                todo!()
            },
        };

        match header.id {
            ProtocolId::Amqp => {
                let idle_time_out = self.idle_time_out
                    .map(|millis| Duration::from_millis(millis as u64));
                let transport = Transport::<_, amqp::Frame>::bind(stream, MIN_MAX_FRAME_SIZE, idle_time_out);
            },
            ProtocolId::Tls => todo!(),
            ProtocolId::Sasl => todo!(),
        }


        todo!()
    }
}

/// A connection on the listener side
#[derive(Debug)]
pub struct ListenerConnection { 
    
}