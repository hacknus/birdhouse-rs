use dioxus::prelude::*;
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
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn init_websocket(&mut self) {
        if *self.ws_connected.read() {
            return;
        }

        if self.ws.read().is_some() {
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
        let socket = WebSocket::new(&format!("{}://{}/ws/tcp", ws_protocol, host))
            .expect("open TCP websocket");

        let mut ir_enabled = self.ir_enabled;
        let mut ir_filter = self.ir_filter_enabled;

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

        let mut ws_connected = self.ws_connected;
        let mut ws_handle = self.ws;

        let on_open = Closure::wrap(Box::new(move |_| {
            ws_connected.set(true);
        }) as Box<dyn FnMut(web_sys::Event)>);

        socket.set_onopen(Some(on_open.as_ref().unchecked_ref()));
        on_open.forget();

        let on_close = Closure::wrap(Box::new(move |_| {
            web_sys::console::warn_1(&"WebSocket closed, reconnecting…".into());
            ws_connected.set(false);
            ws_handle.set(None);
        }) as Box<dyn FnMut(web_sys::CloseEvent)>);

        socket.set_onclose(Some(on_close.as_ref().unchecked_ref()));
        on_close.forget();

        let mut ws_connected = self.ws_connected;
        let mut ws_handle = self.ws;

        let on_error = Closure::wrap(Box::new(move |_| {
            web_sys::console::error_1(&"WebSocket error, reconnecting…".into());
            ws_connected.set(false);
            ws_handle.set(None);
        }) as Box<dyn FnMut(web_sys::Event)>);

        socket.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        on_error.forget();

        socket.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        on_message.forget();

        self.ws.set(Some(socket));
    }
}
