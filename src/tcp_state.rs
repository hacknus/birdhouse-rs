use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{closure::Closure, JsCast};
#[cfg(target_arch = "wasm32")]
use web_sys::{MessageEvent, WebSocket};

#[derive(Clone, Copy)]
pub struct TcpState {
    pub ir_enabled: Signal<bool>,
    pub ir_filter_enabled: Signal<bool>,
    pub is_admin: Signal<bool>,
}

impl TcpState {
    pub fn new() -> Self {
        Self {
            ir_enabled: Signal::new(false),
            ir_filter_enabled: Signal::new(false),
            is_admin: Signal::new(false),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn init_websocket(&self) {
        static WS_INITIALIZED: std::sync::atomic::AtomicBool =
            std::sync::atomic::AtomicBool::new(false);

        if WS_INITIALIZED.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return;
        }

        let window = web_sys::window().expect("browser window");
        let host = window
            .location()
            .host()
            .unwrap_or_else(|_| "127.0.0.1:8080".into());
        let socket = WebSocket::new(&format!("ws://{host}/ws/tcp")).expect("open TCP websocket");

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

        socket.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        on_message.forget();
    }
}