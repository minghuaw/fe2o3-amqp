use std::cmp::min;
use std::collections::BTreeMap;
use std::time::Duration;

use fe2o3_amqp::primitives::UInt;
use fe2o3_types::definitions::{AmqpError, ConnectionError, Error, Handle};
use fe2o3_types::performatives::{Begin, Close, Open};
use slab::Slab;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use futures_util::{Sink, SinkExt, StreamExt};

use crate::error::EngineError;
use crate::transport::amqp::{Frame, FrameBody};
use crate::transport::session::{SessionFrame, SessionHandle, SessionLocalOption};
use crate::transport::Transport;

use super::heartbeat::HeartBeat;

const DEFAULT_CONTROL_CHAN_BUF: usize = 10;
pub const DEFAULT_CONNECTION_MUX_BUFFER_SIZE: usize = u16::MAX as usize;

use super::{ConnectionState, IncomingChannelId, OutgoingChannelId};

pub(crate) enum ConnMuxControl {
    // Open,
    NewSession{
        resp: oneshot::Sender<Result<JoinHandle<Result<(), EngineError>>, EngineError>>,
        option: SessionLocalOption,
    },
    Close,
}

pub struct ConnMuxHandle {
    control: Sender<ConnMuxControl>,
    handle: JoinHandle<Result<(), EngineError>>,

    // Sender to Connection Mux, should be cloned to a new session
    session_tx: Sender<SessionFrame>,
}

impl ConnMuxHandle {
    pub async fn close(&mut self) -> Result<(), EngineError> {
        self.control.send(ConnMuxControl::Close).await?;
        match (&mut self.handle).await {
            Ok(r) => r,
            Err(_) => Err(EngineError::Message("Join Error")),
        }
    }
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
    ) -> Result<ConnMuxHandle, EngineError>
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
        Ok(ConnMuxHandle {
            control: control_tx,
            handle,
            session_tx,
        })
    }

    pub fn pipelined_open<Io>(
        transport: Transport<Io>,
        local_state: ConnectionState,
        local_open: Open,
        buffer_size: usize,
    ) -> Result<ConnMuxHandle, EngineError> {
        todo!()
    }

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
        let max_frame_size = min(
            self.local_open.max_frame_size.0,
            remote_open.max_frame_size.0,
        );
        transport.set_max_frame_size(max_frame_size as usize);

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
        resp: oneshot::Sender<Result<JoinHandle<Result<(), EngineError>>, EngineError>>,
        local_option: SessionLocalOption
    ) -> Result<&ConnectionState, EngineError> {
        // get new entry index
        let entry = self.local_sessions.vacant_entry();
        let key = entry.key();
        
        // check if there is enough
        if key > self.local_open.channel_max.0 as usize {
            resp.send(Err(EngineError::AmqpError(AmqpError::NotAllowed)))
                .map_err(|_| EngineError::Message("Oneshot receiver is already dropped"))?;
            return Err(EngineError::Message("Exceeding max number of channel is not allowed"))
        }

        let outgoing_chan = OutgoingChannelId(key as u16);

        // send Begin frame to remote peer
        let begin = Begin {
            // The remote-channel field of a begin frame MUST be empty 
            // for a locally initiated session, and MUST be set when announcing 
            // the endpoint created as a result of a remotely initiated session.
            remote_channel: None,
            next_outgoing_id: local_option.next_outgoing_id,
            incoming_window: local_option.incoming_window,
            outgoing_window: local_option.outgoing_window,
            handle_max: Handle::from(local_option.handle_max),
            offered_capabilities: local_option.offered_capabilities,
            desired_capabilities: local_option.desired_capabilities,
            properties: local_option.properties,
        };

        // create new session
        // the new session should then send a Begin

        todo!()
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
        let Frame { channel, body } = item?;
        match body {
            FrameBody::Open { performative } => {
                self.handle_open_recv(transport, performative).await
            }
            FrameBody::Close { performative } => {
                self.handle_close_recv(transport, performative).await
            }
            FrameBody::Empty => Ok(&self.local_state),
            _ => todo!(),
        }
    }

    #[inline]
    async fn handle_outgoing<W>(
        &mut self,
        item: Frame,
        writer: &mut W,
    ) -> Result<&ConnectionState, EngineError>
    where
        W: Sink<Frame, Error = EngineError> + Unpin,
    {
        // // get outgoing channel id
        // let chan = OutChanId::from(item.channel());
        // // send frames out
        // if let Err(err) = writer.send(item).await {
        //     if let Some(session) = self.local_sessions.get_mut(chan.0 as usize) {
        //         session.sender_mut()
        //             .send(Err(err)).await
        //             .map_err(|_| EngineError::Message("SendError"))?;
        //     } else {
        //         return Err(err)
        //     }
        // }
        // Ok(&self.local_state)

        // looks like wrong

        todo!()
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
            ConnectionState::Start | ConnectionState::End => return Ok(&self.local_state),
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
                            resp,
                            option,
                        } => self.handle_local_new_session(resp, option).await,
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
            // outgoing frames
            next = self.session_rx.recv() => {
                todo!()
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
