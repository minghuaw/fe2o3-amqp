use std::collections::BTreeMap;

use fe2o3_types::performatives::{Close, Open};
use slab::Slab;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task::JoinHandle;

use futures_util::{Sink, SinkExt, Stream, StreamExt};

use crate::error::EngineError;
use crate::transport::amqp::{Frame, FrameBody};
use crate::transport::session::{SessionFrame, SessionHandle};
use crate::transport::Transport;

const DEFAULT_CONTROL_CHAN_BUF: usize = 10;

use super::{InChanId, OutChanId};

pub enum MuxControl {
    Open(Open),
    NewSession(Option<InChanId>),
    Close(Close),
}

pub struct MuxHandle {
    control: Sender<MuxControl>,
    handle: JoinHandle<Result<(), EngineError>>,
}

impl MuxHandle {
    pub async fn stop(&mut self) -> Result<(), EngineError> {
        // self.control.send(MuxControl::Stop).await
        //     .map_err(|_| EngineError::Message("SendError"))
        todo!()
    }
}

pub struct Multiplexer {
    // Sender to Connection Mux, should be cloned to a new session
    session_tx: Sender<SessionFrame>,
    // Receiver from Session
    session_rx: Receiver<SessionFrame>,
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

    async fn handle_new_session(&mut self, remote_chan: Option<InChanId>) -> Result<(), EngineError> {
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

        // loop {
        //     tokio::select! {
        //         control = self.control.recv() => {
        //             match control {
        //                 Some(control) => {
        //                      match control {
        //                         MuxControl::Open 
        //                         MuxControl::NewSession(remote_chan) => self.handle_new_session(remote_chan).await?,
        //                         MuxControl::Close => todo!(),
        //                      }
        //                 },
        //                 None => {

        //                 }
        //             }
        //         },
        //         incoming = reader.next() => {
        //             match incoming {
        //                 Some(item) => {
        //                     if let Some(control) = self.handle_incoming(item).await? {
        //                         match control {
        //                             MuxControl::Open(open) => {
        //                                 todo!()
        //                             },
        //                             MuxControl::NewSession(remote_chan) => self.handle_new_session(remote_chan).await?,
        //                             MuxControl::Close(close) => return Ok(())
        //                         }
        //                     }
        //                 },
        //                 None => return Err(EngineError::IsClosed)
        //             }
        //         },
        //         outgoing = self.session_rx.recv() => {
        //             match outgoing {
        //                 Some(item) => {
        //                     self.handle_outgoing(item, &mut writer).await?;
        //                 },
        //                 None => {

        //                 }
        //             }
        //         }
        //     }
        // }
        todo!()
    }
}