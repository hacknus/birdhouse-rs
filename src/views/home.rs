use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use std::cell::{Cell, RefCell};
#[cfg(feature = "server")]
use std::collections::HashMap;
#[cfg(feature = "server")]
use std::net::{IpAddr, SocketAddr};
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;
#[cfg(feature = "server")]
use std::sync::Arc;
#[cfg(feature = "server")]
use std::time::{SystemTime, UNIX_EPOCH};
#[cfg(feature = "server")]
use tokio::sync::Mutex;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{closure::Closure, JsCast};
#[cfg(target_arch = "wasm32")]
use web_sys::{MessageEvent, WebSocket};

const CAMERA_SVG: Asset = asset!("/assets/svg/camera.svg");
#[cfg(feature = "server")]
const IMAGE_SAVE_LIMIT_PER_HOUR: usize = 5;

#[cfg(target_arch = "wasm32")]
struct SpectrogramSocketCallbacks {
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _on_open: Closure<dyn FnMut(web_sys::Event)>,
    _on_error: Closure<dyn FnMut(web_sys::ErrorEvent)>,
    _on_close: Closure<dyn FnMut(web_sys::CloseEvent)>,
}

#[cfg(target_arch = "wasm32")]
struct SpectrogramRuntime {
    stop_flag: Rc<Cell<bool>>,
    timeout_id: Rc<RefCell<Option<i32>>>,
    render_loop: Rc<RefCell<Option<Closure<dyn FnMut()>>>>,
    socket_ref: Rc<RefCell<Option<WebSocket>>>,
    socket_callbacks: Rc<RefCell<Option<SpectrogramSocketCallbacks>>>,
}

#[cfg(target_arch = "wasm32")]
fn detach_spectrogram_socket_handlers(socket: &WebSocket) {
    socket.set_onmessage(None);
    socket.set_onopen(None);
    socket.set_onerror(None);
    socket.set_onclose(None);
}

#[cfg(target_arch = "wasm32")]
impl SpectrogramRuntime {
    fn stop(&self) {
        self.stop_flag.set(true);

        if let Some(window) = web_sys::window() {
            if let Some(id) = self.timeout_id.borrow_mut().take() {
                window.clear_timeout_with_handle(id);
            }
        }

        if let Some(socket) = self.socket_ref.borrow_mut().take() {
            detach_spectrogram_socket_handlers(&socket);
            let _ = socket.close();
        }

        self.socket_callbacks.borrow_mut().take();
        self.render_loop.borrow_mut().take();
    }
}

#[cfg(target_arch = "wasm32")]
impl Drop for SpectrogramRuntime {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static SPECTROGRAM_RUNTIME: RefCell<Option<SpectrogramRuntime>> = const { RefCell::new(None) };
}

#[cfg(target_arch = "wasm32")]
fn set_spectrogram_runtime(runtime: SpectrogramRuntime) {
    SPECTROGRAM_RUNTIME.with(|slot| {
        let mut slot = slot.borrow_mut();
        if let Some(existing) = slot.take() {
            existing.stop();
        }
        *slot = Some(runtime);
    });
}

#[cfg(target_arch = "wasm32")]
fn clear_spectrogram_runtime() {
    SPECTROGRAM_RUNTIME.with(|slot| {
        let mut slot = slot.borrow_mut();
        if let Some(existing) = slot.take() {
            existing.stop();
        }
    });
}

#[cfg(feature = "server")]
static IMAGE_SAVE_TRACKER: once_cell::sync::Lazy<Arc<Mutex<HashMap<String, Vec<u64>>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct ImageSaveStatus {
    count_last_hour: usize,
    limit_per_hour: usize,
    is_limited: bool,
}

#[cfg(feature = "server")]
fn extract_real_ip(headers: &dioxus_fullstack::http::HeaderMap) -> Option<IpAddr> {
    for header_name in [
        "cf-connecting-ip",
        "x-real-ip",
        "x-forwarded-for",
        "forwarded",
    ] {
        if let Some(value) = headers.get(header_name).and_then(|v| v.to_str().ok()) {
            if header_name == "x-forwarded-for" {
                let first = value.split(',').next()?.trim();
                if let Ok(ip) = first.parse::<IpAddr>() {
                    return Some(ip);
                }
            } else if let Ok(ip) = value.trim().parse::<IpAddr>() {
                return Some(ip);
            }
        }
    }
    None
}

#[cfg(feature = "server")]
async fn client_ip_key() -> String {
    if let Ok(headers) =
        dioxus_fullstack::FullstackContext::extract::<dioxus_fullstack::http::HeaderMap, _>().await
    {
        if let Some(ip) = extract_real_ip(&headers) {
            return ip.to_string();
        }
    }

    if let Ok(connect_info) = dioxus_fullstack::FullstackContext::extract::<
        dioxus_fullstack::axum::extract::ConnectInfo<SocketAddr>,
        _,
    >()
    .await
    {
        return connect_info.0.ip().to_string();
    }

    "unknown".to_string()
}

#[cfg(feature = "server")]
async fn image_save_status_for_ip(ip_key: &str) -> ImageSaveStatus {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let one_hour_ago = now.saturating_sub(3600);

    let mut tracker = IMAGE_SAVE_TRACKER.lock().await;
    let entries = tracker.entry(ip_key.to_string()).or_default();
    entries.retain(|&ts| ts > one_hour_ago);
    let count_last_hour = entries.len();

    ImageSaveStatus {
        count_last_hour,
        limit_per_hour: IMAGE_SAVE_LIMIT_PER_HOUR,
        is_limited: count_last_hour >= IMAGE_SAVE_LIMIT_PER_HOUR,
    }
}

#[server]
async fn get_image_save_status() -> Result<ImageSaveStatus, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let ip_key = client_ip_key().await;
        return Ok(image_save_status_for_ip(&ip_key).await);
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Not running on server"))
    }
}

#[server]
async fn save_image_to_gallery() -> Result<String, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let ip_key = client_ip_key().await;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut tracker = IMAGE_SAVE_TRACKER.lock().await;
        let one_hour_ago = now.saturating_sub(3600);
        let entries = tracker.entry(ip_key).or_default();
        entries.retain(|&ts| ts > one_hour_ago);

        if entries.len() >= IMAGE_SAVE_LIMIT_PER_HOUR {
            return Err(ServerFnError::new(format!(
                "Too many pictures taken in the last hour (limit: {}).",
                IMAGE_SAVE_LIMIT_PER_HOUR
            )));
        }

        tcp_client::send_command("[CMD] save image")
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to save image: {}", e)))?;

        entries.push(now);
        let count = entries.len();
        return Ok(format!(
            "Image saved successfully. {} images saved in the last hour.",
            count
        ));
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Not running on server"))
    }
}
#[server]
pub async fn get_stream_config() -> Result<StreamConfig, ServerFnError> {
    Ok(StreamConfig {
        stream_url: std::env::var("STREAM_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8889/cam".to_string()),
        websocket_url: std::env::var("WEBSOCKET_URL")
            .unwrap_or_else(|_| "ws://127.0.0.1:8000/ws".to_string()),
        grafana_base_url: std::env::var("GRAFANA_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:3000".to_string()),
        grafana_dashboard: std::env::var("GRAFANA_DASHBOARD")
            .unwrap_or_else(|_| "ad9bp5g/voegeli".to_string()),
        grafana_dashboard_nerds: std::env::var("GRAFANA_DASHBOARD_NERDS")
            .unwrap_or_else(|_| "no-dashboard".to_string()),
    })
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct StreamConfig {
    pub stream_url: String,
    pub websocket_url: String,
    pub grafana_base_url: String,
    pub grafana_dashboard: String,
    pub grafana_dashboard_nerds: String,
}

#[cfg(feature = "server")]
use crate::tcp_client;
use crate::tcp_state;
#[cfg(feature = "server")]
use crate::CURRENT_LUMINOSITY;

const IR_LUX_THRESHOLD: f64 = 300.0;

#[server]
async fn toggle_ir_led(enabled: bool) -> Result<bool, ServerFnError> {
    if enabled {
        let current_lux = {
            let lock = CURRENT_LUMINOSITY
                .read()
                .map_err(|_| ServerFnError::new("Luminosity lock poisoned"))?;
            *lock
        };

        match current_lux {
            Some(lux) if lux < IR_LUX_THRESHOLD => {}
            Some(lux) => {
                return Err(ServerFnError::new(format!(
                    "IR LED can only be enabled below {IR_LUX_THRESHOLD:.0} lux. Current luminosity: {lux:.0} lux."
                )));
            }
            None => {}
        }
    }

    let cmd = if enabled {
        "[CMD] IR ON"
    } else {
        "[CMD] IR OFF"
    };

    tcp_client::send_command(cmd)
        .await
        .map(|_| enabled)
        .map_err(ServerFnError::new)
}

#[server]
async fn get_ir_state() -> Result<bool, ServerFnError> {
    let response = tcp_client::send_command("[CMD] GET IR STATE")
        .await
        .map_err(|e| ServerFnError::new(e))?;

    println!("Received IR state response from TCP: '{:?}'", response);

    let payload = response.to_uppercase();
    for raw_line in payload.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        println!("Checking line for IR state: '{:?}'", line);

        if line.contains("IR STATE IS ON") {
            return Ok(true);
        }

        if line.contains("IR STATE IS OFF") {
            return Ok(false);
        }
    }

    Err(ServerFnError::new(format!(
        "Unexpected IR state response from TCP: {}",
        response
    )))
}

#[server]
async fn get_current_luminosity() -> Result<Option<f64>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let lock = CURRENT_LUMINOSITY
            .read()
            .map_err(|_| ServerFnError::new("Luminosity lock poisoned"))?;
        Ok(*lock)
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Not running on server"))
    }
}

#[cfg(target_arch = "wasm32")]
fn init_webgl_spectrogram(
    canvas_id: &str,
    ws_url: &str,
) -> Result<SpectrogramRuntime, wasm_bindgen::JsValue> {
    use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document
        .get_element_by_id(canvas_id)
        .ok_or("Canvas not found")?
        .dyn_into::<HtmlCanvasElement>()?;

    let ctx = canvas
        .get_context("2d")?
        .ok_or("2D context not supported")?
        .dyn_into::<CanvasRenderingContext2d>()?;

    const HEIGHT: u32 = 256;
    const SECONDS: u32 = 20;
    const FPS: u32 = 30;
    const WIDTH: u32 = SECONDS * FPS;

    canvas.set_width(WIDTH);
    canvas.set_height(HEIGHT);

    const SAMPLE_RATE: f64 = 44100.0;
    const FFT_SIZE: usize = 1024;
    const FREQ_MIN: f64 = 200.0;
    const FREQ_MAX: f64 = 12000.0;

    let mut log_freq_map = vec![0usize; HEIGHT as usize];
    for y in 0..HEIGHT {
        let frac = y as f64 / HEIGHT as f64;
        let freq = FREQ_MIN * (FREQ_MAX / FREQ_MIN).powf(frac);
        let bin = (freq * FFT_SIZE as f64 / SAMPLE_RATE).round() as usize;
        log_freq_map[(HEIGHT - 1 - y) as usize] = bin.min(FFT_SIZE / 2 - 1);
    }

    let log_freq_map = Rc::new(log_freq_map);
    let frame_buffer = Rc::new(RefCell::new(Vec::<Vec<f64>>::new()));
    let socket_ref = Rc::new(RefCell::new(None::<WebSocket>));
    let socket_callbacks = Rc::new(RefCell::new(None::<SpectrogramSocketCallbacks>));
    let stop_flag = Rc::new(Cell::new(false));
    let timeout_id = Rc::new(RefCell::new(None::<i32>));
    let ws_url = ws_url.to_string();

    let connect_websocket = {
        let frame_buffer = frame_buffer.clone();
        let socket_ref = socket_ref.clone();
        let socket_callbacks = socket_callbacks.clone();
        let stop_flag = stop_flag.clone();
        let ws_url = ws_url.clone();

        move || {
            if stop_flag.get() || socket_ref.borrow().is_some() {
                return;
            }

            socket_callbacks.borrow_mut().take();

            let socket = match WebSocket::new(&ws_url) {
                Ok(s) => s,
                Err(e) => {
                    web_sys::console::error_1(
                        &format!("Failed to create WebSocket: {:?}", e).into(),
                    );
                    return;
                }
            };

            let frame_buffer_clone = frame_buffer.clone();

            let on_message = Closure::wrap(Box::new(move |event: MessageEvent| {
                if let Some(text) = event.data().as_string() {
                    if let Ok(fft) = serde_json::from_str::<Vec<f64>>(&text) {
                        let mut buffer = frame_buffer_clone.borrow_mut();
                        if buffer.len() < 10 {
                            buffer.push(fft);
                        }
                    }
                }
            }) as Box<dyn FnMut(_)>);

            let on_open = Closure::wrap(Box::new(move |_: web_sys::Event| {
                web_sys::console::log_1(&"Spectrogram WebSocket connected".into());
            }) as Box<dyn FnMut(web_sys::Event)>);

            let on_error = Closure::wrap(Box::new(move |e: web_sys::ErrorEvent| {
                web_sys::console::error_1(&format!("Spectrogram WebSocket error: {:?}", e).into());
            }) as Box<dyn FnMut(web_sys::ErrorEvent)>);

            let socket_ref_close = socket_ref.clone();
            let socket_for_close = socket.clone();
            let on_close = Closure::wrap(Box::new(move |_: web_sys::CloseEvent| {
                web_sys::console::warn_1(&"Spectrogram WebSocket closed, will retry".into());
                detach_spectrogram_socket_handlers(&socket_for_close);
                *socket_ref_close.borrow_mut() = None;
            }) as Box<dyn FnMut(web_sys::CloseEvent)>);

            socket.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
            socket.set_onopen(Some(on_open.as_ref().unchecked_ref()));
            socket.set_onerror(Some(on_error.as_ref().unchecked_ref()));
            socket.set_onclose(Some(on_close.as_ref().unchecked_ref()));

            *socket_ref.borrow_mut() = Some(socket);
            *socket_callbacks.borrow_mut() = Some(SpectrogramSocketCallbacks {
                _on_message: on_message,
                _on_open: on_open,
                _on_error: on_error,
                _on_close: on_close,
            });
        }
    };

    connect_websocket();

    let ctx_render = ctx.clone();
    let canvas_render = canvas.clone();
    let log_freq_map_render = log_freq_map.clone();
    let pixel_buffer = Rc::new(RefCell::new(vec![0u8; HEIGHT as usize * 4]));

    let render_loop = Rc::new(RefCell::new(None::<Closure<dyn FnMut()>>));
    let render_loop_clone = render_loop.clone();
    let stop_flag_render = stop_flag.clone();
    let timeout_id_render = timeout_id.clone();
    let socket_ref_render = socket_ref.clone();
    let frame_buffer_render = frame_buffer.clone();

    *render_loop.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        if stop_flag_render.get() {
            return;
        }

        // Check connection and reconnect if needed
        if socket_ref_render.borrow().is_none() {
            connect_websocket();
        }

        // Check if we have data and pop it in a separate scope
        let fft_data = frame_buffer_render.borrow_mut().pop();

        let delay = if let Some(fft) = fft_data {
            // Process data at 30 FPS
            let _ = ctx_render.draw_image_with_html_canvas_element(&canvas_render, -1.0, 0.0);

            let mut pixels = pixel_buffer.borrow_mut();
            for y in 0..HEIGHT as usize {
                let bin = log_freq_map_render[y];
                let v = fft.get(bin).copied().unwrap_or(0.0);

                let norm = ((v + 6.0) / 6.0).clamp(0.0, 1.0);
                let enhanced = norm.powf(1.2);

                // let contrast = 0.5;
                // let contrasted = ((enhanced - 0.5) * contrast + 0.5).clamp(0.0, 1.0);

                let c = (enhanced * 255.0) as u8;

                let idx = y * 4;
                pixels[idx] = c;
                pixels[idx + 1] = (c as f64 * 0.7) as u8;
                pixels[idx + 2] = (c as u16 + 80).min(255) as u8;
                pixels[idx + 3] = 255;
            }

            if let Ok(image_data) = ImageData::new_with_u8_clamped_array_and_sh(
                wasm_bindgen::Clamped(&pixels),
                1,
                HEIGHT,
            ) {
                let _ = ctx_render.put_image_data(&image_data, (WIDTH - 1) as f64, 0.0);
            }

            33 // 30 FPS when rendering
        } else {
            1000 // 1 second when idle (no data)
        };

        let window = web_sys::window().unwrap();
        if let Ok(id) = window.set_timeout_with_callback_and_timeout_and_arguments_0(
            render_loop_clone
                .borrow()
                .as_ref()
                .unwrap()
                .as_ref()
                .unchecked_ref(),
            delay,
        ) {
            *timeout_id_render.borrow_mut() = Some(id);
        }
    }) as Box<dyn FnMut()>));

    let window = web_sys::window().unwrap();
    let id = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        render_loop
            .borrow()
            .as_ref()
            .unwrap()
            .as_ref()
            .unchecked_ref(),
        0,
    )?;
    *timeout_id.borrow_mut() = Some(id);

    ctx.set_fill_style(&"black".into());
    ctx.fill_rect(0.0, 0.0, WIDTH as f64, HEIGHT as f64);

    Ok(SpectrogramRuntime {
        stop_flag,
        timeout_id,
        render_loop,
        socket_ref,
        socket_callbacks,
    })
}

pub fn Home() -> Element {
    let config = use_resource(|| async move { get_stream_config().await.ok() });
    let mut save_status_refresh = use_signal(|| 0u64);
    let save_status = use_resource(move || {
        let _ = save_status_refresh();
        async move { get_image_save_status().await.ok() }
    });
    let tcp_state = use_context::<tcp_state::TcpState>();
    let mut ir_enabled = tcp_state.ir_enabled;
    let mut saving = use_signal(|| false);
    let mut ir_request_id = use_signal(|| 0u64);
    let mut ir_feedback = use_signal(|| None::<String>);
    let mut lux_refresh = use_signal(|| 0u64);
    let luminosity = use_resource(move || {
        let _ = lux_refresh();
        async move { get_current_luminosity().await.ok().flatten() }
    });

    // Load initial states in background without blocking render
    use_resource(move || async move {
        if let Ok(state) = get_ir_state().await {
            ir_enabled.set(state);
        }
    });

    use_effect(move || {
        let _handle = spawn(async move {
            #[cfg(target_arch = "wasm32")]
            gloo_timers::future::sleep(std::time::Duration::from_secs(10)).await;

            loop {
                lux_refresh += 1;
                #[cfg(target_arch = "wasm32")]
                gloo_timers::future::sleep(std::time::Duration::from_secs(10)).await;
            }
        });
    });

    let config_value = config.read();
    let Some(Some(cfg)) = config_value.as_ref() else {
        return rsx! {
            div {
                class: "min-h-screen w-full flex items-center justify-center bg-slate-900 text-white",
                "Loading..."
            }
        };
    };
    let save_status_value = save_status.read();
    let save_status_current = save_status_value.as_ref().and_then(|s| s.as_ref().cloned());
    let is_save_limited = save_status_current
        .as_ref()
        .map(|s| s.is_limited)
        .unwrap_or(false);
    let save_tooltip = if is_save_limited {
        "too many pictures taken in the last hour"
    } else {
        "save image"
    };
    let current_lux = luminosity.read().as_ref().and_then(|lux| *lux);
    let can_enable_ir = current_lux
        .map(|lux| lux < IR_LUX_THRESHOLD)
        .unwrap_or(true);
    let ir_toggle_disabled = !ir_enabled() && !can_enable_ir;
    let ir_label = "Light".to_string();
    let ir_tooltip = if ir_toggle_disabled {
        current_lux
            .map(|lux| {
                format!("IR LED can only be enabled at night. Current illuminance: {lux:.0} lux")
            })
            .unwrap_or_else(|| "toggle IR LED".to_string())
    } else {
        "toggle IR LED".to_string()
    };

    let stream_url = cfg.stream_url.clone();
    #[cfg(target_arch = "wasm32")]
    let ws_url = cfg.websocket_url.clone();

    #[cfg(target_arch = "wasm32")]
    {
        let mut spec_initialized = use_signal(|| false);
        let ws_url_clone = ws_url.clone();

        use_effect(move || {
            if !spec_initialized() {
                match init_webgl_spectrogram("spectrogram", &ws_url_clone) {
                    Ok(runtime) => {
                        set_spectrogram_runtime(runtime);
                        spec_initialized.set(true);
                    }
                    Err(e) => {
                        web_sys::console::error_1(
                            &format!("Failed to init spectrogram: {:?}", e).into(),
                        );
                    }
                }
            }
        });

        use_drop(move || {
            clear_spectrogram_runtime();
        });
    }

    rsx! {
        section {
            class: "min-h-screen w-full flex flex-col items-center bg-slate-900 gap-6 py-4",

            div {
                class: "w-full max-w-7xl flex flex-row flex-nowrap items-center justify-center gap-6 px-4 py-2 overflow-x-auto",
                div {
                    class: "flex items-center gap-2",
                    label {
                        class: "text-white font-small whitespace-nowrap",
                        "{ir_label}"
                    }
                    button {
                        class: format!(
                            "relative inline-flex h-6 w-12 items-center rounded-full transition-colors {}",
                            if ir_toggle_disabled {
                                "bg-gray-700 opacity-60 cursor-not-allowed"
                            } else if ir_enabled() {
                                "bg-blue-500"
                            } else {
                                "bg-gray-600"
                            }
                        ),
                        disabled: ir_toggle_disabled,
                        title: ir_tooltip,
                        onclick: move |_| {
                            if ir_toggle_disabled {
                                return;
                            }
                            let previous_state = ir_enabled();
                            let new_state = !previous_state;
                            let request_id = ir_request_id() + 1;
                            ir_request_id.set(request_id);
                            ir_enabled.set(new_state);
                            ir_feedback.set(None);

                            let mut ir_request_id_timeout = ir_request_id;
                            let mut ir_enabled_timeout = ir_enabled;
                            spawn(async move {
                                #[cfg(target_arch = "wasm32")]
                                gloo_timers::future::sleep(std::time::Duration::from_secs(5)).await;

                                if ir_request_id_timeout() == request_id {
                                    ir_enabled_timeout.set(previous_state);
                                    ir_request_id_timeout.set(0);
                                }
                            });

                            let mut ir_request_id_ack = ir_request_id;
                            let mut ir_enabled_ack = ir_enabled;
                            spawn(async move {
                                if ir_request_id_ack() != request_id {
                                    return;
                                }

                                match toggle_ir_led(new_state).await {
                                    Ok(state) => {
                                        if ir_request_id_ack() == request_id {
                                            ir_enabled_ack.set(state);
                                            ir_request_id_ack.set(0);
                                            lux_refresh += 1;
                                        }
                                    }
                                    Err(err) => {
                                        if ir_request_id_ack() == request_id {
                                            ir_enabled_ack.set(previous_state);
                                            ir_request_id_ack.set(0);
                                            ir_feedback.set(Some(err.to_string()));
                                            lux_refresh += 1;
                                        }
                                    }
                                }
                            });
                        },
                        span {
                            class: format!(
                                "inline-block h-4 w-4 transform rounded-full bg-white transition-transform {}",
                                if ir_enabled() { "translate-x-7" } else { "translate-x-1" }
                            )
                        }
                    }
                }
                if let Some(msg) = ir_feedback() {
                    p { class: "text-xs text-amber-300 whitespace-nowrap", "{msg}" }
                }
                div {
                    class: "flex items-center gap-4",
                    label {
                        class: "text-white font-small whitespace-nowrap",
                        "Save Image"
                    }
                    // p {
                    //     class: "text-xs text-slate-300 whitespace-nowrap",
                    //     "{save_count_text}"
                    // }

                   button {
                        class: format!(
                            "px-4 py-0.5 rounded-lg transition-colors {}",
                            if is_save_limited {
                                "bg-gray-500 text-gray-300 cursor-not-allowed"
                            } else if saving() {
                                "bg-blue-500 hover:bg-blue-600 active:bg-blue-700"
                            } else {
                                "bg-white hover:bg-gray-200"
                            }
                        ),
                        disabled: saving() || is_save_limited,
                        title: save_tooltip,
                        onclick: move |_| {
                            if saving() || is_save_limited {
                                return;
                            }

                            saving.set(true);

                            spawn(async move {
                                match save_image_to_gallery().await {
                                    Ok(_msg) => {
                                        #[cfg(target_arch = "wasm32")]
                                        web_sys::console::log_1(&_msg.into());
                                    }
                                    Err(_e) => {
                                        #[cfg(target_arch = "wasm32")]
                                        web_sys::console::error_1(&format!("Error: {}", _e).into());
                                    }
                                }

                                // Return to white
                                saving.set(false);
                                save_status_refresh += 1;
                            });
                        },

                        img {
                            src: CAMERA_SVG,
                            class: "w-6 h-6",
                            alt: "Save image"
                        }
                    }
                }
            }

            div {
                class: "w-full flex flex-col items-center gap-6 px-4",
                style: "--content-width: min(100%, 1280px); --stream-height: calc(var(--content-width) * 9 / 16); --spec-height: calc(var(--content-width) * 4 / 16);",
                iframe {
                    src: stream_url,
                    style: "height: var(--stream-height); aspect-ratio: 16 / 9; width: var(--content-width);",
                    class: "rounded-lg bg-gray-800 shadow-lg",
                    allow: "camera;autoplay;encrypted-media",
                    allowfullscreen: true,
                }
                canvas {
                    id: "spectrogram",
                    style: "height: var(--spec-height); aspect-ratio: 16 / 4; width: var(--content-width);",
                    class: "rounded-lg bg-black shadow-lg",
                }
            }

            div {
                class: "w-full max-w-7xl grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4",

                iframe {
                    src: format!("{}/d-solo/{}?orgId=1&timezone=browser&refresh=5s&panelId=panel-7&__feature.dashboardSceneSolo=true&from=now-24h&to=now", cfg.grafana_base_url, cfg.grafana_dashboard),
                    class: "w-full h-64 rounded-lg border-2 border-slate-700",
                }

                iframe {
                    src: format!("{}/d-solo/{}?orgId=1&timezone=browser&refresh=5s&panelId=panel-8&__feature.dashboardSceneSolo=true&from=now-24h&to=now", cfg.grafana_base_url, cfg.grafana_dashboard),
                    class: "w-full h-64 rounded-lg border-2 border-slate-700",
                }

                iframe {
                    src: format!("{}/d-solo/{}?orgId=1&timezone=browser&refresh=5s&panelId=panel-9&__feature.dashboardSceneSolo=true&from=now-24h&to=now", cfg.grafana_base_url, cfg.grafana_dashboard),
                    class: "w-full h-64 rounded-lg border-2 border-slate-700",
                }

                iframe {
                    src: format!("{}/d-solo/{}?orgId=1&timezone=browser&refresh=5s&panelId=panel-10&__feature.dashboardSceneSolo=true&from=now-24h&to=now", cfg.grafana_base_url, cfg.grafana_dashboard),
                    class: "w-full h-64 rounded-lg border-2 border-slate-700",
                }

                iframe {
                    src: format!("{}/d-solo/{}?orgId=1&timezone=browser&refresh=5s&panelId=panel-12&__feature.dashboardSceneSolo=true&from=now-24h&to=now", cfg.grafana_base_url, cfg.grafana_dashboard),
                    class: "w-full h-64 rounded-lg border-2 border-slate-700",
                }

                iframe {
                    src: format!("{}/d-solo/{}?orgId=1&timezone=browser&refresh=5s&panelId=panel-11&__feature.dashboardSceneSolo=true&from=now-24h&to=now", cfg.grafana_base_url, cfg.grafana_dashboard),
                    class: "w-full h-64 rounded-lg border-2 border-slate-700",
                }
            }
        }
    }
}
