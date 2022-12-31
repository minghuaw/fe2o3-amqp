//! WebSocket in wasm with web-sys

use std::{io, collections::VecDeque, rc::Rc, cell::RefCell, task::Waker};

use futures_util::{Sink, Stream};
use tungstenite::protocol::CloseFrame;
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::{CloseEvent, ErrorEvent, MessageEvent, WebSocket};

use crate::{Error, WsMessage};

enum ReadyState {
    Connecting,
    Open,
    Closing,
    Closed,
}

impl TryFrom<u16> for ReadyState {
    type Error = ();

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ReadyState::Connecting),
            1 => Ok(ReadyState::Open),
            2 => Ok(ReadyState::Closing),
            3 => Ok(ReadyState::Closed),
            _ => Err(()),
        }
    }
}

/// A wrapper over [`web_sys::WebSocket`] that implements `Stream` and `Sink`.
#[derive(Debug)]
pub struct WasmWebSocketStream {
    inner: WebSocket,
    // Atomic synchronization primitives are not available in wasm, keep everything in a single
    // thread
    queue: Rc<RefCell<VecDeque<Result<WsMessage, tungstenite::Error>>>>,
    waker: Rc<RefCell<Option<Waker>>>,
    _on_message_callback: Closure<dyn FnMut(MessageEvent)>,
    _on_close_callback: Closure<dyn FnMut(CloseEvent)>,
    _on_error_callback: Closure<dyn FnMut(ErrorEvent)>,
}

impl From<WasmWebSocketStream> for super::WebSocketStream<WasmWebSocketStream> {
    fn from(stream: WasmWebSocketStream) -> Self {
        super::WebSocketStream {
            inner: stream,
            current_binary: None,
        }
    }
}

impl WasmWebSocketStream {
    /// Connect websocket
    pub async fn connect(addr: impl AsRef<str>) -> Result<Self, Error> {
        let ws = WebSocket::new_with_str(addr.as_ref(), super::SEC_WEBSOCKET_PROTOCOL_AMQP)
            .map_err(|_| {
                Error::Io(io::Error::new(
                    io::ErrorKind::Other,
                    "Failed to create WebSocket with web-sys",
                ))
            })?;

        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        let (open_tx, mut open_rx) = futures_channel::oneshot::channel();
        let on_open_callback = Closure::once_into_js(move || {
            let _ = open_tx.send(());
        });
        ws.set_onopen(Some(on_open_callback.as_ref().unchecked_ref()));

        let (err_tx, mut err_rx) = futures_channel::oneshot::channel();
        let on_error_callback = Closure::once_into_js(move |event: ErrorEvent| {
            let _ = err_tx.send(event);
        });
        ws.set_onerror(Some(on_error_callback.as_ref().unchecked_ref()));

        let result = futures_util::select! {
            _ = open_rx => Ok(()),
            event = err_rx => Err(Error::Io(io::Error::new(io::ErrorKind::Other, format!("Failed to connect to WebSocket: {:?}", event)))),
        };
        ws.set_onopen(None);
        ws.set_onerror(None);
        result?;

        let waker: Rc<RefCell<Option<Waker>>> = Rc::new(RefCell::new(None));
        let queue = Rc::new(RefCell::new(VecDeque::new()));

        let waker_clone = waker.clone();
        let queue_clone = queue.clone();
        let on_message_callback = Closure::wrap(Box::new(move |event: MessageEvent| {
            let result = WsMessage::try_from(event);
            queue_clone.borrow_mut().push_back(result);
            if let Some(_waker) = waker_clone.borrow_mut().take() {
                _waker.wake();
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(on_message_callback.as_ref().unchecked_ref()));

        let waker_clone = waker.clone();
        let queue_clone = queue.clone();
        let on_error_callback = Closure::wrap(Box::new(move |event: ErrorEvent| {
            let result = Err(tungstenite::Error::Io(io::Error::new(
                io::ErrorKind::Other,
                format!("WebSocket error: {:?}", event),
            )));
            queue_clone.borrow_mut().push_back(result);
            if let Some(_waker) = waker_clone.borrow_mut().take() {
                _waker.wake();
            }
        }) as Box<dyn FnMut(ErrorEvent)>);
        ws.set_onerror(Some(on_error_callback.as_ref().unchecked_ref()));

        let waker_clone = waker.clone();
        let queue_clone = queue.clone();
        let on_close_callback = Closure::wrap(Box::new(move |event: CloseEvent| {
            let message = tungstenite::Message::Close(Some(CloseFrame {
                code: event.code().into(),
                reason: event.reason().into(),
            }));
            let result = Ok(WsMessage(message));
            queue_clone.borrow_mut().push_back(result);
            if let Some(_waker) = waker_clone.borrow_mut().take() {
                _waker.wake();
            }
        }) as Box<dyn FnMut(CloseEvent)>);
        ws.set_onclose(Some(on_close_callback.as_ref().unchecked_ref()));

        Ok(Self {
            inner: ws,
            queue,
            waker,
            _on_message_callback: on_message_callback,
            _on_close_callback: on_close_callback,
            _on_error_callback: on_error_callback,
        })
    }
}

impl super::WebSocketStream<WasmWebSocketStream> {
    /// Connect websocket over wasm
    pub async fn connect(addr: impl AsRef<str>) -> Result<Self, Error> {
        let stream = WasmWebSocketStream::connect(addr).await?;
        Ok(Self::from(stream))
    }
}

impl TryFrom<MessageEvent> for WsMessage {
    type Error = tungstenite::Error;

    fn try_from(event: MessageEvent) -> Result<Self, Self::Error> {
        let data = event.data();
        let data = data.dyn_into::<js_sys::ArrayBuffer>().map_err(|_| {
            tungstenite::Error::Io(io::Error::new(
                io::ErrorKind::InvalidData,
                "Only Binary data is supported",
            ))
        })?;
        let data = js_sys::Uint8Array::new(&data);
        let data = data.to_vec();
        Ok(WsMessage(tungstenite::Message::Binary(data)))
    }
}

impl Stream for WasmWebSocketStream {
    type Item = Result<WsMessage, tungstenite::Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let mut queue = self.queue.borrow_mut();
        if let Some(item) = queue.pop_front() {
            return std::task::Poll::Ready(Some(item));
        }
        *self.waker.borrow_mut() = Some(cx.waker().clone());
        std::task::Poll::Pending
    }
}

impl Sink<WsMessage> for WasmWebSocketStream {
    type Error = tungstenite::Error;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        match self.inner.ready_state().try_into() {
            Ok(ReadyState::Open) => std::task::Poll::Ready(Ok(())),
            _ => std::task::Poll::Ready(Err(tungstenite::Error::ConnectionClosed)),
        }
    }

    fn start_send(self: std::pin::Pin<&mut Self>, item: WsMessage) -> Result<(), Self::Error> {
        match self.inner.ready_state().try_into() {
            Ok(ReadyState::Open) => match item.0 {
                tungstenite::Message::Text(item) => self
                    .inner
                    .send_with_str(&item)
                    .map_err(|_| tungstenite::Error::ConnectionClosed),
                tungstenite::Message::Binary(item) => self
                    .inner
                    .send_with_u8_array(&item)
                    .map_err(|_| tungstenite::Error::ConnectionClosed),
                tungstenite::Message::Close(frame) => match frame {
                    Some(frame) => self
                        .inner
                        .close_with_code_and_reason(frame.code.into(), &frame.reason)
                        .map_err(|_| tungstenite::Error::ConnectionClosed),
                    None => self
                        .inner
                        .close()
                        .map_err(|_| tungstenite::Error::ConnectionClosed),
                },
                tungstenite::Message::Ping(_)
                | tungstenite::Message::Pong(_)
                | tungstenite::Message::Frame(_) => Err(tungstenite::Error::Io(io::Error::new(
                    io::ErrorKind::Other,
                    "Sending Ping, Pong and Frame is not supported",
                ))),
            },
            _ => Err(tungstenite::Error::ConnectionClosed),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner
            .close()
            .map_err(|_| tungstenite::Error::AlreadyClosed)?;
        std::task::Poll::Ready(Ok(()))
    }
}
