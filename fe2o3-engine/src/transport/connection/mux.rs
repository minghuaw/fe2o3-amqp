use std::cmp::min;
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::time::Duration;

use serde_amqp::primitives::UInt;
use fe2o3_types::definitions::{AmqpError, ConnectionError, Error, Handle};
use fe2o3_types::performatives::{Begin, ChannelMax, Close, Open};
use slab::Slab;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use futures_util::{Sink, SinkExt, StreamExt};

use crate::error::EngineError;
use crate::transport::amqp::{Frame, FrameBody};
use crate::transport::session::{NonSessionFrame, NonSessionFrameBody, SessionFrame, SessionFrameBody, SessionHandle};
use crate::transport::Transport;

use super::heartbeat::HeartBeat;

pub(crate) const DEFAULT_CONTROL_CHAN_BUF: usize = 10;
pub const DEFAULT_CONNECTION_MUX_BUFFER_SIZE: usize = u16::MAX as usize;

use super::{Connection, ConnectionState, IncomingChannelId, OutgoingChannelId};

pub(crate) enum ConnMuxControl {
    // Open,
    NewSession {
        handle: SessionHandle,
        resp: oneshot::Sender<Result<OutgoingChannelId, EngineError>>,
    },
    Close,
}

pub struct ConnMux {
    local_state: ConnectionState,
    local_open: Open,
    local_sessions: Slab<SessionHandle>, // Slab is indexed with OutgoingChannel

    remote_state: Option<ConnectionState>,
    remote_open: Option<Open>,
    remote_sessions: BTreeMap<IncomingChannelId, OutgoingChannelId>, // maps from remote channel id to local channel id
    // remote_header: ProtocolHeader,
    heartbeat: HeartBeat,

    // mutual limitations
    channel_max: u16,

    // Receiver from Session
    session_rx: Receiver<SessionFrame>,
    // Receiver from Connection
    control: Receiver<ConnMuxControl>,
}

impl ConnMux {
    // Initial exchange of protocol header / connection header should be
    // handled before spawning the Mux
    pub async fn open<Io>(
        mut transport: Transport<Io>,
        local_state: ConnectionState,
        local_open: Open,
        buffer_size: usize,
    ) -> Result<Connection, EngineError>
    where
        Io: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        match local_state {
            ConnectionState::HeaderExchange => {}
            s @ _ => return Err(EngineError::UnexpectedConnectionState(s)),
        };

        let (session_tx, session_rx) = mpsc::channel(buffer_size);
        let (control_tx, control_rx) = mpsc::channel(DEFAULT_CONTROL_CHAN_BUF);
        let local_sessions = Slab::new(); // TODO: pre-allocate capacity
        let channel_max = local_open.channel_max.0;

        let mut mux = Self {
            local_state,
            local_open,
            local_sessions,
            remote_open: None,
            remote_state: None,
            remote_sessions: BTreeMap::new(),
            // remote_header,
            heartbeat: HeartBeat::never(),
            // session_tx,
            channel_max,
            session_rx,
            control: control_rx,
        };
        println!(">>> Debug: open() - Openning");
        // Send Open
        mux.handle_open_send(&mut transport).await?;
        // Recv Open
        if let Err(err) = mux.recv_open(&mut transport).await {
            if let EngineError::ConnectionError(conn_err) = err {
                let local_error = Error::from(conn_err);
                mux.handle_close_send(&mut transport, Some(local_error))
                    .await?;
            } else {
                return Err(err);
            }
        };

        // spawn mux loop
        let handle = tokio::spawn(mux.mux_loop(transport));
        Ok(Connection {
            mux: control_tx,
            handle,
            session_tx,
        })
    }

    pub fn pipelined_open<Io>(
        transport: Transport<Io>,
        local_state: ConnectionState,
        local_open: Open,
        buffer_size: usize,
    ) -> Result<Connection, EngineError> {
        todo!()
    }
}

impl ConnMux {
    #[inline]
    async fn handle_unexpected_drop(&mut self) -> Result<&ConnectionState, EngineError> {
        todo!()
    }

    #[inline]
    async fn handle_unexpected_eof(&mut self) -> Result<&ConnectionState, EngineError> {
        todo!()
    }

    #[inline]
    async fn handle_open_send<Io>(
        &mut self,
        transport: &mut Transport<Io>,
    ) -> Result<&ConnectionState, EngineError>
    where
        Io: AsyncRead + AsyncWrite + Unpin,
    {
        println!(">>> Debug: handle_open_send()");
        println!(">>> Debug: {:?}", self.local_open);

        // State transition (Fig. 2.23)
        // Return early to avoid sending Open when it's not supposed to
        match &self.local_state {
            ConnectionState::HeaderExchange => self.local_state = ConnectionState::OpenSent,
            ConnectionState::OpenReceived => self.local_state = ConnectionState::Opened,
            ConnectionState::HeaderSent => self.local_state = ConnectionState::OpenPipe,
            s @ _ => return Err(EngineError::UnexpectedConnectionState(s.clone())),
        }
        let frame = Frame::new(
            0u16,
            FrameBody::Open {
                performative: self.local_open.clone(),
            },
        );
        transport.send(frame).await?;
        println!("Sent frame");
        Ok(&self.local_state)
    }

    #[inline]
    async fn recv_open<Io>(
        &mut self,
        transport: &mut Transport<Io>,
    ) -> Result<&ConnectionState, EngineError>
    where
        Io: AsyncRead + AsyncWrite + Unpin,
    {
        let frame = match transport.next().await {
            Some(frame) => frame?,
            None => return self.handle_unexpected_eof().await,
        };
        let remote_open = match frame.body {
            FrameBody::Open { performative } => performative,
            _ => return Err(EngineError::ConnectionError(ConnectionError::FramingError)),
        };
        self.handle_open_recv(transport, remote_open).await
    }

    #[inline]
    async fn handle_open_recv<Io>(
        &mut self,
        transport: &mut Transport<Io>,
        remote_open: Open,
    ) -> Result<&ConnectionState, EngineError>
    where
        Io: AsyncRead + AsyncWrite + Unpin,
    {
        println!(">>> Debug: handle_open_recv()");
        println!(">>> Debug: {:?}", &remote_open);

        match &self.local_state {
            ConnectionState::HeaderExchange => self.local_state = ConnectionState::OpenReceived,
            ConnectionState::OpenSent => self.local_state = ConnectionState::Opened,
            ConnectionState::ClosePipe => self.local_state = ConnectionState::CloseSent,
            _ => return Err(EngineError::illegal_state()),
        }
        // FIXME: is there anything we need to check?
        // set max_frame_size to mutually acceptable
        let max_frame_size = min(
            self.local_open.max_frame_size.0,
            remote_open.max_frame_size.0,
        );
        transport.set_max_frame_size(max_frame_size as usize);

        // set channel_max to mutually acceptable
        let channel_max = min(self.local_open.channel_max.0, remote_open.channel_max.0);
        self.channel_max = channel_max;

        // Set heartbeat here because in pipelined-open, the Open frame
        // may be recved after mux loop is started
        match &remote_open.idle_time_out {
            Some(millis) => {
                let period = Duration::from_millis(*millis as u64);
                self.heartbeat = HeartBeat::new(period);
            }
            None => self.heartbeat = HeartBeat::never(),
        };
        self.remote_open = Some(remote_open);

        Ok(&self.local_state)
    }

    #[inline]
    async fn handle_close_send<Io>(
        &mut self,
        transport: &mut Transport<Io>,
        local_error: Option<Error>,
    ) -> Result<&ConnectionState, EngineError>
    where
        Io: AsyncRead + AsyncWrite + Unpin,
    {
        println!(">>> Debug: handle_close_send()");
        // check state first, and return early if state is wrong
        match &self.local_state {
            ConnectionState::Opened => self.local_state = ConnectionState::CloseSent,
            ConnectionState::CloseReceived => self.local_state = ConnectionState::End,
            ConnectionState::OpenSent => self.local_state = ConnectionState::ClosePipe,
            ConnectionState::OpenPipe => self.local_state = ConnectionState::OpenClosePipe,
            s @ _ => return Err(EngineError::UnexpectedConnectionState(s.clone())),
        };

        let frame = Frame::new(
            0u16,
            FrameBody::Close {
                performative: Close { error: local_error },
            },
        );
        transport.send(frame).await?;
        Ok(&self.local_state)
    }

    #[inline]
    async fn handle_close_recv<Io>(
        &mut self,
        transport: &mut Transport<Io>,
        _remote_close: Close,
    ) -> Result<&ConnectionState, EngineError>
    where
        Io: AsyncRead + AsyncWrite + Unpin,
    {
        println!(">>> Debug: handle_close_recv()");
        // TODO: how to handle or log remote close?
        match &self.local_state {
            ConnectionState::Opened => {
                self.local_state = ConnectionState::CloseReceived;
                // respond with a Close
                let frame = Frame::new(
                    0u16,
                    FrameBody::Close {
                        performative: Close { error: None },
                    },
                );
                transport.send(frame).await?;
                // transition to End state
                self.local_state = ConnectionState::End;
            }
            ConnectionState::CloseSent => {
                self.local_state = ConnectionState::End;
            }
            // other states are invalid
            _ => return Err(EngineError::illegal_state()),
        };
        Ok(&self.local_state)
    }

    /// TODO: Simply create a new session and let the session takes care
    /// of sending Begin?
    #[inline]
    async fn handle_local_new_session(
        &mut self,
        handle: SessionHandle,
        resp: oneshot::Sender<Result<OutgoingChannelId, EngineError>>,
    ) -> Result<&ConnectionState, EngineError> {
        match &self.local_state {
            ConnectionState::Start 
            | ConnectionState::HeaderSent
            | ConnectionState::HeaderReceived
            | ConnectionState::HeaderExchange
            | ConnectionState::CloseSent
            | ConnectionState::Discarding
            | ConnectionState::End => {
                return Err(EngineError::illegal_state())
            },
            // TODO: what about pipelined open?
            _ => {}
        }

        // get new entry index
        let entry = self.local_sessions.vacant_entry();
        let channel = entry.key();

        // check if there is enough
        if channel > self.channel_max as usize {
            resp.send(Err(EngineError::AmqpError(AmqpError::NotAllowed)))
                .map_err(|_| EngineError::Message("Oneshot receiver is already dropped"))?;
            return Err(EngineError::Message(
                "Exceeding max number of channel is not allowed",
            ));
        }

        let outgoing_chan = OutgoingChannelId(channel as u16);
        entry.insert(handle);
        resp.send(Ok(outgoing_chan))
            .map_err(|_| EngineError::Message("Oneshot receiver is already dropped"))?;
        Ok(&self.local_state)
    }

    #[inline]
    async fn handle_incoming<Io>(
        &mut self,
        transport: &mut Transport<Io>,
        item: Result<Frame, EngineError>,
    ) -> Result<&ConnectionState, EngineError>
    where
        Io: AsyncRead + AsyncWrite + Unpin,
    {
        println!(">>> Debug: handle_incoming()");

        // local_state should be checked in each sub-handler
        match item?.try_into() {
            Ok(frame) => self.handle_incoming_session_frame(frame).await,
            Err(frame) => {
                let NonSessionFrame{channel: _, body} = frame;
                match body {
                    NonSessionFrameBody::Open{performative} => {
                        self.handle_open_recv(transport, performative).await
                    },
                    NonSessionFrameBody::Close{performative} => {
                        self.handle_close_recv(transport, performative).await
                    },
                    NonSessionFrameBody::Empty => Ok(&self.local_state),
                }
            }
        }
    }

    #[inline]
    async fn handle_incoming_session_frame(
        &mut self,
        frame: SessionFrame
    ) -> Result<&ConnectionState, EngineError> {
        println!(">>> Debug: handle_incoming_session_frame()");
        match &self.local_state {
            ConnectionState::Opened => { }, // TODO: is there any other state that should accept session frames?
            _ => return Err(EngineError::illegal_state())
        }

        match &frame.body { 
            SessionFrameBody::Begin{performative} => {
                match performative.remote_channel {
                    Some(local_channel) => {
                        // let local_channel = performative.remote_channel
                        //     .ok_or_else(|| EngineError::not_allowed())?;
                        let remote_channel = frame.channel;
        
                        let key = IncomingChannelId(remote_channel);
                        let value = OutgoingChannelId(local_channel);
        
                        // check whether there is existing sessions
                        if self.remote_sessions.contains_key(&key) {
                            return Err(EngineError::not_allowed()) // FIXME: what error should be returned here?
                        }
        
                        self.remote_sessions.insert(key, value);
        
                        // forward begin frame to session mux
                        let handle = self.local_sessions.get_mut(local_channel as usize)
                            .ok_or_else(|| EngineError::not_found())?;
                        handle.sender_mut().send(Ok(frame)).await?;
                    },
                    None => {
                        todo!()
                    }
                }
            },
            SessionFrameBody::End{performative: _} => {
                // stop session mux?
            },
            _ => {
                let remote_channel = IncomingChannelId(frame.channel);
                let local_channel = self.remote_sessions.get(&remote_channel)
                    .ok_or_else(|| EngineError::Message("Unexpected remote channel from frame"))?;
                let handle = self.local_sessions.get_mut(local_channel.0 as usize)
                    .ok_or_else(|| EngineError::not_found())?;
                handle.sender_mut().send(Ok(frame)).await?;
            }
        }
        
        Ok(&self.local_state)
    }

    #[inline]
    async fn handle_outgoing<W>(
        &mut self,
        writer: &mut W,
        item: SessionFrame,
    ) -> Result<&ConnectionState, EngineError>
    where
        W: Sink<Frame, Error = EngineError> + Unpin,
    {
        println!(">>> Debug: handle_outgoing");

        // TODO: check local state
        // match self.local_state {

        // }

        let frame: Frame = item.into();
        writer.send(frame).await?;
        Ok(&self.local_state)
    }

    #[inline]
    async fn handle_error<Io>(
        &mut self,
        transport: &mut Transport<Io>,
        error: EngineError,
    ) -> Result<&ConnectionState, EngineError>
    where
        Io: AsyncRead + AsyncWrite + Unpin,
    {
        match error {
            EngineError::IdleTimeout => {
                println!("Idle timeout");
                todo!()
            }
            EngineError::MaxFrameSizeExceeded => {
                let local_error = Error::from(ConnectionError::FramingError);
                self.handle_close_send(transport, Some(local_error)).await
            }
            EngineError::AmqpError(amqp_err) => match amqp_err {
                AmqpError::IllegalState => {
                    todo!()
                }
                _ => todo!(),
            },
            _ => Ok(&self.local_state),
        }
    }

    #[inline]
    async fn handle_heartbeat<Io>(
        &mut self,
        transport: &mut Transport<Io>,
    ) -> Result<&ConnectionState, EngineError>
    where
        Io: AsyncRead + AsyncWrite + Unpin,
    {
        match &self.local_state {
            ConnectionState::Start 
            | ConnectionState::CloseSent
            | ConnectionState::End => return Ok(&self.local_state),
            _ => {}
        }

        let frame = Frame::empty();
        transport.send(frame).await?;
        Ok(&self.local_state)
    }

    #[inline]
    async fn mux_loop_inner<Io>(
        &mut self,
        transport: &mut Transport<Io>,
    ) -> Result<&ConnectionState, EngineError>
    where
        Io: AsyncRead + AsyncWrite + Unpin,
    {
        println!(">>> Debug: Connection State: {:?}", &self.local_state);

        tokio::select! {
            // local controls
            control = self.control.recv() => {
                match control {
                    Some(control) => match control {
                        // MuxControl::Open => self.handle_open_send(transport).await,
                        ConnMuxControl::NewSession{
                            handle,
                            resp,
                        } => self.handle_local_new_session(handle, resp).await,
                        ConnMuxControl::Close => self.handle_close_send(transport, None).await,
                    },
                    None => {
                        self.handle_unexpected_drop().await
                    }
                }
            },
            // incoming frames
            next = transport.next() => {
                match next {
                    Some(item) => self.handle_incoming(transport, item).await,
                    None => self.handle_unexpected_eof().await
                }
            },
            // outgoing frames from session
            next = self.session_rx.recv() => {
                match next {
                    Some(item) => self.handle_outgoing(transport, item).await,
                    None => self.handle_unexpected_eof().await
                }
            },
            // heartbeat
            _ = self.heartbeat.next() => {
                self.handle_heartbeat(transport).await
            }
        }
    }

    async fn mux_loop<Io>(mut self, mut transport: Transport<Io>) -> Result<(), EngineError>
    where
        Io: AsyncRead + AsyncWrite + Send + Unpin,
    {
        loop {
            let running = match self.mux_loop_inner(&mut transport).await {
                Ok(running) => running,
                Err(error) => match self.handle_error(&mut transport, error).await {
                    Ok(r) => r,
                    Err(err) => {
                        let local_error =
                            Error::new(AmqpError::InternalError, Some(err.to_string()), None);
                        match self
                            .handle_close_send(&mut transport, Some(local_error))
                            .await
                        {
                            Ok(r) => r,
                            Err(err) => {
                                println!("!!! Internal error: {:?}. Likely unrecoverable. Closing the connection", err);
                                self.local_state = ConnectionState::End;
                                &self.local_state
                            }
                        }
                    }
                },
            };
            if let ConnectionState::End = running {
                return Ok(());
            }
        }
    }
}
