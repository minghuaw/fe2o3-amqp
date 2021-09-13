use std::cmp::min;
use std::collections::BTreeMap;
use std::str::EncodeUtf16;

use fe2o3_types::performatives::{Close, Open};
use slab::Slab;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task::JoinHandle;

use futures_util::{Sink, SinkExt, Stream, StreamExt};

use crate::error::EngineError;
use crate::transport::amqp::{Frame, FrameBody};
use crate::transport::protocol_header::ProtocolHeader;
use crate::transport::session::{SessionFrame, SessionHandle};
use crate::transport::Transport;

const DEFAULT_CONTROL_CHAN_BUF: usize = 10;
pub const DEFAULT_CONNECTION_MUX_BUFFER_SIZE: usize = u16::MAX as usize;

use super::{ConnectionState, InChanId, OutChanId};

pub enum MuxControl {
    Open,
    // NewSession(Option<InChanId>),
    Close,
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

    pub fn control_mut(&mut self) -> &mut Sender<MuxControl> {
        &mut self.control
    }
}

pub struct Mux {
    local_state: ConnectionState,
    local_open: Open,
    local_sessions: Slab<SessionHandle>,

    remote_state: Option<ConnectionState>,
    remote_open: Option<Open>,
    remote_header: ProtocolHeader,

    // Sender to Connection Mux, should be cloned to a new session
    session_tx: Sender<SessionFrame>,
    // Receiver from Session
    session_rx: Receiver<SessionFrame>,
    // Receiver from Connection
    control: Receiver<MuxControl>,
    in_out_map: BTreeMap<InChanId, OutChanId>,
}

impl Mux {
    // Initial exchange of protocol header / connection header should be 
    // handled before spawning the Mux
    pub fn spawn<Io>(
        transport: Transport<Io>, 
        local_state: ConnectionState, 
        local_open: Open, 
        remote_header: ProtocolHeader, 
        buffer_size: usize
    ) -> Result<MuxHandle, EngineError>
    where
        Io: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        // check connection state
        match &local_state {
            ConnectionState::HeaderExchange => {},
            ConnectionState::HeaderSent => {}, // TODO: Pipelined open
            _ => return Err(EngineError::Message("Expecting local_state to be ConnectionState::HeaderExchange"))
        }

        let (session_tx, session_rx) = mpsc::channel(buffer_size);
        let (control_tx, control_rx) = mpsc::channel(DEFAULT_CONTROL_CHAN_BUF);
        let local_sessions = Slab::new(); // TODO: pre-allocate capacity

        let mux = Self {
            local_state,
            local_open,
            remote_open: None,
            remote_state: None,
            remote_header,
            session_tx,
            session_rx,
            control: control_rx,
            local_sessions,
            in_out_map: BTreeMap::new()
        };
        let handle = tokio::spawn(mux.mux_loop(transport));
        Ok(MuxHandle {
            control: control_tx,
            handle
        })
    }

    async fn handle_unexpected_drop(&mut self) -> Result<(), EngineError> {
        todo!()
    }

    async fn handle_unexpected_eof(&mut self) -> Result<(), EngineError> {
        todo!()
    }

    async fn handle_open<Io>(&mut self, transport: &mut Transport<Io>, remote_open: Option<Open>) -> Result<(), EngineError> 
    where 
        Io: AsyncRead + AsyncWrite + Unpin,
    {
        println!("hanlde open");

        match remote_open {
            Some(open) => {
                match &self.local_state {
                    ConnectionState::HeaderExchange => self.local_state = ConnectionState::OpenReceived,
                    ConnectionState::OpenSent => self.local_state = ConnectionState::Opened,
                    ConnectionState::ClosePipe => self.local_state = ConnectionState::CloseSent,
                    s @ _ => return Err(EngineError::UnexpectedConnectionState(s.clone()))
                }
                // FIXME: is there anything we need to check?
                let max_frame_size = min(self.local_open.max_frame_size.0, open.max_frame_size.0);
                transport.set_max_frame_size(max_frame_size as usize);
                self.remote_open = Some(open);
                Ok(())
            }
            // handling a locally initiated open
            None => {
                let frame = Frame::new(
                    0u16, 
                    FrameBody::Open{ performative: self.local_open.clone() }
                );
                transport.send(frame).await?;
                println!("Sent frame");
                // TODO: State transition (Fig. 2.23)
                match &self.local_state {
                    ConnectionState::HeaderExchange => self.local_state = ConnectionState::OpenSent,
                    ConnectionState::OpenReceived => self.local_state = ConnectionState::Opened,
                    ConnectionState::HeaderSent => self.local_state = ConnectionState::OpenPipe,
                    s @ _ => return Err(EngineError::UnexpectedConnectionState(s.clone()))
                }
                Ok(())
            },
        }
    }

    async fn handle_close(&mut self, remote_close: Option<Close>) -> Result<(), EngineError> {
        todo!()
    }

    async fn handle_new_session(&mut self) -> Result<(), EngineError> {
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

    async fn mux_loop<Io>(mut self, mut transport: Transport<Io>) -> Result<(), EngineError> 
    where 
        Io: AsyncRead + AsyncWrite + Send + Unpin
    {
        // let (mut writer, mut reader) = transport.split();

        loop {
            tokio::select! {
                // local controls
                control = self.control.recv() => {
                    match control {
                        Some(control) => {
                            match control {
                                MuxControl::Open => self.handle_open(&mut transport, None).await?,
                                MuxControl::Close => self.handle_close(None).await?
                            }
                        },
                        None => return self.handle_unexpected_drop().await
                    }
                },
                next = transport.next() => {
                    match next {
                        Some(item) => {
                            let Frame{channel, body} = item?;
                            match body {
                                FrameBody::Open{performative} => self.handle_open(&mut transport, Some(performative)).await?,
                                FrameBody::Close{performative} => self.handle_close(Some(performative)).await?,
                                _ => todo!()
                            }
                        },
                        None => return self.handle_unexpected_eof().await
                    }
                }
                next = self.session_rx.recv() => {
                    todo!()
                }
            }
        }
    }
}