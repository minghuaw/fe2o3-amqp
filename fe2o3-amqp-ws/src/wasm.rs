//! WebSocket in wasm with web-sys

use web_sys::{WebSocket, MessageEvent};

struct WasmWebSocketStream(WebSocket);