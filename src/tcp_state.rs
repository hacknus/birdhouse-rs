use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use gloo_timers::callback::Interval;
#[cfg(target_arch = "wasm32")]
use uuid::Uuid;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{closure::Closure, JsCast};
#[cfg(target_arch = "wasm32")]
use web_sys::{MessageEvent, WebSocket};

#[cfg(target_arch = "wasm32")]
struct ViewerWsCallbacks {
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _on_open: Closure<dyn FnMut(web_sys::Event)>,
    _on_close: Closure<dyn FnMut(web_sys::CloseEvent)>,
    _on_error: Closure<dyn FnMut(web_sys::Event)>,
}

#[cfg(target_arch = "wasm32")]
fn parse_bool_from_state_payload(
    payload: &str,
    on_contains: &[&str],
    off_contains: &[&str],
    on_exact: &[&str],
    off_exact: &[&str],
) -> Option<bool> {
    for raw_line in payload.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if on_exact.contains(&line) || on_contains.iter().any(|p| line.contains(p)) {
            return Some(true);
        }

        if off_exact.contains(&line) || off_contains.iter().any(|p| line.contains(p)) {
            return Some(false);
        }
    }

    None
}

#[derive(Clone)]
pub struct TcpState {
    pub ir_enabled: Signal<bool>,
    pub ir_filter_enabled: Signal<bool>,
    pub is_admin: Signal<bool>,
    pub ws_connected: Signal<bool>,
    #[cfg(target_arch = "wasm32")]
    pub ws: Signal<Option<WebSocket>>,
    #[cfg(target_arch = "wasm32")]
    ws_callbacks: Signal<Option<ViewerWsCallbacks>>,
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
            ws_callbacks: Signal::new(None),
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
            let ws_callbacks = self.ws_callbacks;

            let reconnect = Interval::new(3_000, move || {
                if ws_handle.read().is_none() {
                    open_viewer_websocket(
                        ir_enabled,
                        ir_filter,
                        ws_connected,
                        ws_handle,
                        ws_callbacks,
                    );
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
                self.ws_callbacks,
            );
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn detach_viewer_socket_handlers(socket: &WebSocket) {
    socket.set_onopen(None);
    socket.set_onclose(None);
    socket.set_onerror(None);
    socket.set_onmessage(None);
}

#[cfg(target_arch = "wasm32")]
fn open_viewer_websocket(
    mut ir_enabled: Signal<bool>,
    mut ir_filter: Signal<bool>,
    mut ws_connected: Signal<bool>,
    mut ws_handle: Signal<Option<WebSocket>>,
    mut ws_callbacks: Signal<Option<ViewerWsCallbacks>>,
) {
    if ws_handle.read().is_some() {
        return;
    }

    ws_callbacks.set(None);

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
            if let Some(state) = parse_bool_from_state_payload(
                &payload,
                &["IR LED STATE: ON", "IR STATE IS ON"],
                &["IR LED STATE: OFF", "IR STATE IS OFF"],
                &["IR ON", "1"],
                &["IR OFF", "0"],
            ) {
                ir_enabled.set(state);
            }

            if let Some(state) = parse_bool_from_state_payload(
                &payload,
                &["IR FILTER STATE: ON", "IR FILTER STATE IS ON"],
                &["IR FILTER STATE: OFF", "IR FILTER STATE IS OFF"],
                &["IR FILTER ON"],
                &["IR FILTER OFF"],
            ) {
                ir_filter.set(state);
            }
        }
    }) as Box<dyn FnMut(_)>);

    let on_open = Closure::wrap(Box::new(move |_| {
        ws_connected.set(true);
    }) as Box<dyn FnMut(web_sys::Event)>);

    let socket_for_close = socket.clone();
    let on_close = Closure::wrap(Box::new(move |_| {
        web_sys::console::warn_1(&"WebSocket closed, reconnecting…".into());
        detach_viewer_socket_handlers(&socket_for_close);
        ws_connected.set(false);
        ws_handle.set(None);
    }) as Box<dyn FnMut(web_sys::CloseEvent)>);

    let socket_for_error = socket.clone();
    let on_error = Closure::wrap(Box::new(move |_| {
        web_sys::console::error_1(&"WebSocket error, reconnecting…".into());
        detach_viewer_socket_handlers(&socket_for_error);
        let _ = socket_for_error.close();
        ws_connected.set(false);
        ws_handle.set(None);
    }) as Box<dyn FnMut(web_sys::Event)>);

    socket.set_onopen(Some(on_open.as_ref().unchecked_ref()));
    socket.set_onclose(Some(on_close.as_ref().unchecked_ref()));
    socket.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    socket.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

    ws_callbacks.set(Some(ViewerWsCallbacks {
        _on_message: on_message,
        _on_open: on_open,
        _on_close: on_close,
        _on_error: on_error,
    }));

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
