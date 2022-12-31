//! WebSocket in wasm with web-sys

use std::{io, task::Waker};

use futures_util::{Stream, Sink};
use pin_project_lite::pin_project;
use tokio::sync::mpsc::UnboundedReceiver;
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::{CloseEvent, ErrorEvent, MessageEvent, WebSocket};

use crate::WsMessage;

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

pin_project! {
    /// A wrapper over [`web_sys::WebSocket`] that implements `Stream` and `Sink`.
    #[derive(Debug)]
    pub struct WasmWebSocketStream {
        inner: WebSocket,
        #[pin]
        receiver: UnboundedReceiver<Result<WsMessage, tungstenite::Error>>,
        _on_message_callback: Closure<dyn FnMut(MessageEvent)>,
        _on_close_callback: Closure<dyn FnMut(CloseEvent)>,
        _on_error_callback: Closure<dyn FnMut(ErrorEvent)>,
    }
}

impl WasmWebSocketStream {
    // pub async fn connect()
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
        let mut this = self.project();
        this.receiver.poll_recv(cx)
    }
}

impl Sink<WsMessage> for WasmWebSocketStream {
    type Error = tungstenite::Error;

    fn poll_ready(self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        match self.inner.ready_state().try_into() {
            Ok(ReadyState::Open) => std::task::Poll::Ready(Ok(())),
            _ => std::task::Poll::Ready(Err(tungstenite::Error::ConnectionClosed))
        }
    }

    fn start_send(self: std::pin::Pin<&mut Self>, item: WsMessage) -> Result<(), Self::Error> {
        match self.inner.ready_state().try_into() {
            Ok(ReadyState::Open) => match item.0 {
                tungstenite::Message::Text(item) => self.inner.send_with_str(&item).map_err(|_| tungstenite::Error::ConnectionClosed),
                tungstenite::Message::Binary(item) => self.inner.send_with_u8_array(&item).map_err(|_| tungstenite::Error::ConnectionClosed),
                tungstenite::Message::Close(frame) => match frame {
                    Some(frame) => self.inner.close_with_code_and_reason(frame.code.into(), &frame.reason).map_err(|_| tungstenite::Error::ConnectionClosed),
                    None => self.inner.close().map_err(|_| tungstenite::Error::ConnectionClosed),
                },
                tungstenite::Message::Ping(_)
                | tungstenite::Message::Pong(_)
                | tungstenite::Message::Frame(_) => Err(tungstenite::Error::Io(io::Error::new(
                    io::ErrorKind::Other,
                    "Sending Ping, Pong and Frame is not supported",
                ))),
            },
            _ => Err(tungstenite::Error::ConnectionClosed)
        }
    }

    fn poll_flush(self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_close(self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.close().map_err(|_| tungstenite::Error::AlreadyClosed)?;
        std::task::Poll::Ready(Ok(()))
    }
}
