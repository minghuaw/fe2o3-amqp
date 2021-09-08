use std::collections::BTreeMap;

use futures_util::stream::Next;
use slab::Slab;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;

use futures_util::{Sink, SinkExt, Stream, StreamExt};

use crate::error::EngineError;
use crate::transport::amqp::{Frame, FrameBody};
use crate::transport::session::SessionHandle;
use crate::transport::transport::Transport;

const DEFAULT_CONTROL_CHAN_BUF: usize = 10;

use super::{InChanId, OutChanId};

pub enum MuxControl {
    NewSession(Option<InChanId>),
    Stop
}

pub struct MuxHandle {
    control: Sender<MuxControl>,
    handle: JoinHandle<Result<(), EngineError>>,
}

impl MuxHandle {
    pub async fn stop(&mut self) -> Result<(), EngineError> {
        self.control.send(MuxControl::Stop).await
            .map_err(|_| EngineError::Message("SendError"))
    }
}

pub struct Multiplexer {
    // Sender to Connection Mux
    session_tx: Sender<Frame>,
    // Receiver from Session
    session_rx: Receiver<Frame>,
    // Receiver from Connection
    control: Receiver<MuxControl>,
    local_sessions: Slab<SessionHandle>,
    in_out_map: BTreeMap<InChanId, OutChanId>,
}

impl Multiplexer {
    pub fn spawn<Io>(transport: Transport<Io>, buf_size: usize) -> MuxHandle 
    where
        Io: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        let (session_tx, session_rx) = mpsc::channel(buf_size);
        let (control_tx, control_rx) = mpsc::channel(DEFAULT_CONTROL_CHAN_BUF);
        let local_sessions = Slab::new(); // TODO: pre-allocate capacity

        let mux = Multiplexer {
            session_tx,
            session_rx,
            control: control_rx,
            local_sessions,
            in_out_map: BTreeMap::new()
        };
        let handle = tokio::spawn(mux.mux_loop(transport));
        MuxHandle {
            control: control_tx,
            handle
        }
    }

    async fn handle_new_session(&mut self, in_chan: Option<InChanId>) -> Result<(), EngineError> {
        todo!()
        // get new entry index
        // create new session
        // the new session should then send a Begin
    }

    async fn handle_incoming(
        &mut self,
        item: Result<Frame, EngineError>,
    ) -> Result<Option<MuxControl>, EngineError> {
        let frame = item?; 

        // match frame {
            
        // }
        todo!()
    }

    async fn handle_outgoing<W>(
        &mut self,
        item: Frame,
        writer: &mut W,
    ) -> Result<(), EngineError> 
    where 
        W: Sink<Frame, Error = EngineError> + Unpin
    {
        // get outgoing channel id
        let chan = OutChanId::from(item.channel());
        // send frames out
        if let Err(err) = writer.send(item).await {
            if let Some(session) = self.local_sessions.get_mut(chan.0 as usize) {
                session.sender_mut()
                    .send(Err(err)).await
                    .map_err(|_| EngineError::Message("SendError"))?;
            } else {
                return Err(err)
            }
        }
        Ok(())
    }

    async fn mux_loop<Io>(mut self, transport: Transport<Io>) -> Result<(), EngineError> 
    where 
        Io: AsyncRead + AsyncWrite + Send + Unpin
    {
        let (mut writer, mut reader) = transport.split();

        loop {
            tokio::select! {
                control = self.control.recv() => {
                    match control {
                        Some(control) => {
                             match control {
                                MuxControl::NewSession(in_chan) => {
                                    self.handle_new_session(in_chan).await?;
                                    
                                },
                                MuxControl::Stop => {
                                    return Ok(())
                                }
                             }
                        },
                        None => {

                        }
                    }
                },
                incoming = reader.next() => {
                    match incoming {
                        Some(item) => {
                            if let Some(control) = self.handle_incoming(item).await? {
                                match control {
                                    MuxControl::NewSession(in_chan) => self.handle_new_session(in_chan).await?,
                                    MuxControl::Stop => return Ok(())
                                }
                            }
                        },
                        None => return Err(EngineError::IsClosed)
                    }
                },
                outgoing = self.session_rx.recv() => {
                    match outgoing {
                        Some(item) => {
                            self.handle_outgoing(item, &mut writer).await?;
                        },
                        None => {

                        }
                    }
                }
            }
        }
    }
}