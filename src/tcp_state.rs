use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use gloo_timers::callback::Interval;
#[cfg(target_arch = "wasm32")]
use uuid::Uuid;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{closure::Closure, JsCast};
#[cfg(target_arch = "wasm32")]
use web_sys::{MessageEvent, WebSocket};

#[derive(Clone)]
pub struct TcpState {
    pub ir_enabled: Signal<bool>,
    pub ir_filter_enabled: Signal<bool>,
    pub is_admin: Signal<bool>,
    pub ws_connected: Signal<bool>,
    #[cfg(target_arch = "wasm32")]
    pub ws: Signal<Option<WebSocket>>,
    #[cfg(target_arch = "wasm32")]
    pub heartbeat: Signal<Option<Interval>>,
    #[cfg(target_arch = "wasm32")]
    pub reconnect: Signal<Option<Interval>>,
}

impl TcpState {
    pub fn new() -> Self {
        Self {
            ir_enabled: Signal::new(false),
            ir_filter_enabled: Signal::new(false),
            is_admin: Signal::new(false),
            ws_connected: Signal::new(false),
            #[cfg(target_arch = "wasm32")]
            ws: Signal::new(None),
            #[cfg(target_arch = "wasm32")]
            heartbeat: Signal::new(None),
            #[cfg(target_arch = "wasm32")]
            reconnect: Signal::new(None),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn init_websocket(&mut self) {
        if self.heartbeat.read().is_none() {
            let ws_handle = self.ws;
            let interval = Interval::new(5_000, move || {
                if let Some(ws) = ws_handle.read().as_ref() {
                    let _ = ws.send_with_str("hb");
                }
            });
            self.heartbeat.set(Some(interval));
        }

        if self.reconnect.read().is_none() {
            let ir_enabled = self.ir_enabled;
            let ir_filter = self.ir_filter_enabled;
            let ws_connected = self.ws_connected;
            let ws_handle = self.ws;

            let reconnect = Interval::new(3_000, move || {
                if ws_handle.read().is_none() {
                    open_viewer_websocket(ir_enabled, ir_filter, ws_connected, ws_handle);
                }
            });

            self.reconnect.set(Some(reconnect));
        }

        if self.ws.read().is_none() {
            open_viewer_websocket(
                self.ir_enabled,
                self.ir_filter_enabled,
                self.ws_connected,
                self.ws,
            );
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn open_viewer_websocket(
    mut ir_enabled: Signal<bool>,
    mut ir_filter: Signal<bool>,
    mut ws_connected: Signal<bool>,
    mut ws_handle: Signal<Option<WebSocket>>,
) {
    if ws_handle.read().is_some() {
        return;
    }

    let window = web_sys::window().expect("browser window");
    let protocol = window
        .location()
        .protocol()
        .unwrap_or_else(|_| "http:".into());
    let host = window
        .location()
        .host()
        .unwrap_or_else(|_| "127.0.0.1:8080".into());

    let ws_protocol = if protocol == "https:" { "wss" } else { "ws" };
    let session_id = get_or_create_session_id();
    web_sys::console::log_1(&format!("Session ID: {}", session_id).into());

    let socket = match WebSocket::new(&format!(
        "{}://{}/ws/tcp?role=viewer&session_id={}",
        ws_protocol, host, session_id
    )) {
        Ok(ws) => ws,
        Err(err) => {
            web_sys::console::error_1(&format!("WebSocket open failed: {:?}", err).into());
            return;
        }
    };

    let on_message = Closure::wrap(Box::new(move |event: MessageEvent| {
        if let Some(text) = event.data().as_string() {
            let payload = text.to_uppercase();
            match () {
                _ if payload.contains("IR LED STATE: ON")
                    || payload.contains("IR STATE IS ON")
                    || payload.contains("IR ON") =>
                {
                    ir_enabled.set(true);
                }
                _ if payload.contains("IR LED STATE: OFF")
                    || payload.contains("IR STATE IS OFF")
                    || payload.contains("IR OFF") =>
                {
                    ir_enabled.set(false);
                }
                _ if payload.contains("IR FILTER STATE: ON")
                    || payload.contains("IR FILTER STATE IS ON")
                    || payload.contains("IR FILTER ON") =>
                {
                    ir_filter.set(true);
                }
                _ if payload.contains("IR FILTER STATE: OFF")
                    || payload.contains("IR FILTER STATE IS OFF")
                    || payload.contains("IR FILTER OFF") =>
                {
                    ir_filter.set(false);
                }
                _ => {}
            }
        }
    }) as Box<dyn FnMut(_)>);

    let on_open = Closure::wrap(Box::new(move |_| {
        ws_connected.set(true);
    }) as Box<dyn FnMut(web_sys::Event)>);

    let on_close = Closure::wrap(Box::new(move |_| {
        web_sys::console::warn_1(&"WebSocket closed, reconnecting…".into());
        ws_connected.set(false);
        ws_handle.set(None);
    }) as Box<dyn FnMut(web_sys::CloseEvent)>);

    let on_error = Closure::wrap(Box::new(move |_| {
        web_sys::console::error_1(&"WebSocket error, reconnecting…".into());
        ws_connected.set(false);
        ws_handle.set(None);
    }) as Box<dyn FnMut(web_sys::Event)>);

    socket.set_onopen(Some(on_open.as_ref().unchecked_ref()));
    socket.set_onclose(Some(on_close.as_ref().unchecked_ref()));
    socket.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    socket.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

    on_open.forget();
    on_close.forget();
    on_error.forget();
    on_message.forget();

    ws_handle.set(Some(socket));
}

#[cfg(target_arch = "wasm32")]
fn get_or_create_session_id() -> String {
    let key = "birdhouse_session_id";

    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(existing)) = storage.get_item(key) {
                if !existing.is_empty() {
                    return existing;
                }
            }

            let new_id = Uuid::new_v4().to_string();
            let _ = storage.set_item(key, &new_id);
            return new_id;
        }
    }

    Uuid::new_v4().to_string()
}
