use dioxus::dioxus_core::Task;
use dioxus::document::eval;
use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{closure::Closure, JsCast};
#[cfg(target_arch = "wasm32")]
use web_sys::{MessageEvent, WebSocket};

#[server]
async fn get_stream_config() -> Result<StreamConfig, ServerFnError> {
    Ok(StreamConfig {
        stream_url: std::env::var("STREAM_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8889/cam".to_string()),
        websocket_url: std::env::var("WEBSOCKET_URL")
            .unwrap_or_else(|_| "ws://127.0.0.1:8000/ws".to_string()),
        grafana_base_url: std::env::var("GRAFANA_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:3000".to_string()),
        grafana_dashboard: std::env::var("GRAFANA_DASHBOARD")
            .unwrap_or_else(|_| "ad9bp5g/voegeli".to_string()),
    })
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct StreamConfig {
    stream_url: String,
    websocket_url: String,
    grafana_base_url: String,
    grafana_dashboard: String,
}

#[cfg(feature = "server")]
use crate::tcp_client;
use crate::tcp_state;

#[server]
async fn toggle_ir_led(enabled: bool) -> Result<bool, ServerFnError> {
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
async fn toggle_ir_filter(enabled: bool) -> Result<bool, ServerFnError> {
    let cmd = if enabled {
        "[CMD] IR FILTER ON"
    } else {
        "[CMD] IR FILTER OFF"
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
    let is_on = response.to_lowercase().contains("on") || response.contains("1");
    Ok(is_on)
}

#[server]
async fn get_admin_feature_state() -> Result<bool, ServerFnError> {
    let response = tcp_client::send_command("[CMD] GET IR FILTER STATE")
        .await
        .map_err(|e| ServerFnError::new(e))?;
    let is_on = response.to_lowercase().contains("on") || response.contains("1");
    Ok(is_on)
}

#[server]
async fn is_admin() -> Result<bool, ServerFnError> {
    // TODO: Implement actual admin check (e.g., check session, JWT token, etc.)
    // For now, return false
    Ok(false)
}

#[cfg(target_arch = "wasm32")]
fn init_webgl_spectrogram(canvas_id: &str, ws_url: &str) -> Result<(), wasm_bindgen::JsValue> {
    use web_sys::{HtmlCanvasElement, CanvasRenderingContext2d, ImageData};
    use wasm_bindgen::JsCast;
    use std::rc::Rc;
    use std::cell::RefCell;

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
    let ws_url = ws_url.to_string();
    let last_data_time = Rc::new(RefCell::new(js_sys::Date::now()));

    let connect_websocket = {
        let frame_buffer = frame_buffer.clone();
        let socket_ref = socket_ref.clone();
        let last_data_time = last_data_time.clone();
        let ws_url = ws_url.clone();

        move || {
            if socket_ref.borrow().is_some() {
                return;
            }

            let socket = match WebSocket::new(&ws_url) {
                Ok(s) => s,
                Err(e) => {
                    web_sys::console::error_1(&format!("Failed to create WebSocket: {:?}", e).into());
                    return;
                }
            };

            let frame_buffer_clone = frame_buffer.clone();
            let last_data_clone = last_data_time.clone();

            let on_message = Closure::wrap(Box::new(move |event: MessageEvent| {
                if let Some(text) = event.data().as_string() {
                    if let Ok(fft) = serde_json::from_str::<Vec<f64>>(&text) {
                        let mut buffer = frame_buffer_clone.borrow_mut();
                        if buffer.len() < 10 {
                            buffer.push(fft);
                        }
                        *last_data_clone.borrow_mut() = js_sys::Date::now();
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
            let on_close = Closure::wrap(Box::new(move |_: web_sys::CloseEvent| {
                web_sys::console::warn_1(&"Spectrogram WebSocket closed, will retry".into());
                *socket_ref_close.borrow_mut() = None;
            }) as Box<dyn FnMut(web_sys::CloseEvent)>);

            socket.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
            socket.set_onopen(Some(on_open.as_ref().unchecked_ref()));
            socket.set_onerror(Some(on_error.as_ref().unchecked_ref()));
            socket.set_onclose(Some(on_close.as_ref().unchecked_ref()));

            on_message.forget();
            on_open.forget();
            on_error.forget();
            on_close.forget();

            *socket_ref.borrow_mut() = Some(socket);
        }
    };

    connect_websocket();

    let ctx_render = ctx.clone();
    let canvas_render = canvas.clone();
    let log_freq_map_render = log_freq_map.clone();
    let pixel_buffer = Rc::new(RefCell::new(vec![0u8; HEIGHT as usize * 4]));

    let render_loop = Rc::new(RefCell::new(None::<Closure<dyn FnMut()>>));
    let render_loop_clone = render_loop.clone();

    *render_loop.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        // Check connection and reconnect if needed
        if socket_ref.borrow().is_none() {
            connect_websocket();
        }

        // Check if we have data and pop it in a separate scope
        let fft_data = frame_buffer.borrow_mut().pop();
        let has_data = fft_data.is_some();

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
                HEIGHT
            ) {
                let _ = ctx_render.put_image_data(&image_data, (WIDTH - 1) as f64, 0.0);
            }

            33 // 30 FPS when rendering
        } else {
            1000 // 1 second when idle (no data)
        };

        let window = web_sys::window().unwrap();
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
            render_loop_clone.borrow().as_ref().unwrap().as_ref().unchecked_ref(),
            delay
        );
    }) as Box<dyn FnMut()>));

    let window = web_sys::window().unwrap();
    window.set_timeout_with_callback_and_timeout_and_arguments_0(
        render_loop.borrow().as_ref().unwrap().as_ref().unchecked_ref(),
        0
    )?;

    ctx.set_fill_style(&"black".into());
    ctx.fill_rect(0.0, 0.0, WIDTH as f64, HEIGHT as f64);

    Ok(())
}

pub fn Home() -> Element {
    let mut config = use_resource(|| async move { get_stream_config().await.ok() });
    let mut _ws_task = use_signal(|| None::<Task>);
    let tcp_ws_task = use_signal(|| None::<Task>);

    let mut tcp_state = use_context::<tcp_state::TcpState>();
    let mut ir_enabled = tcp_state.ir_enabled;
    let mut ir_filter_enabled = tcp_state.ir_filter_enabled;
    let mut is_admin_user = tcp_state.is_admin;

    // Load initial states in background without blocking render
    use_resource(move || async move {
        if let Ok(state) = get_ir_state().await {
            ir_enabled.set(state);
        }
    });

    use_resource(move || async move {
        if let Ok(state) = get_admin_feature_state().await {
            ir_filter_enabled.set(state);
        }
    });

    use_resource(move || async move {
        if let Ok(admin) = is_admin().await {
            is_admin_user.set(admin);
        }
    });

    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        if !*tcp_state.ws_connected.read() {
            spawn(async move {
                gloo_timers::future::sleep(std::time::Duration::from_millis(500)).await;
                tcp_state.init_websocket();
            });
        }
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

    let stream_url = cfg.stream_url.clone();
    let ws_url = cfg.websocket_url.clone();

    #[cfg(target_arch = "wasm32")]
    {
        let mut initialized = use_signal(|| false);
        let ws_url_clone = ws_url.clone();  // Clone the websocket_url from config

        use_effect(move || {
            if !initialized() {
                if let Err(e) = init_webgl_spectrogram("spectrogram", &ws_url_clone) {
                    web_sys::console::error_1(&format!("Failed to init spectrogram: {:?}", e).into());
                }
                initialized.set(true);
            }
        });
    }


    rsx! {
            section {
                class: "min-h-screen w-full flex flex-col items-center bg-slate-900 gap-6 py-4",

                // Toggle switches container
                div {
                    class: "w-full max-w-7xl flex flex-row flex-nowrap items-center justify-center gap-6 px-4 py-2 overflow-x-auto",
                    div {
                        class: "flex items-center gap-2",
                        label {
                            class: "text-white font-small whitespace-nowrap",
                            "IR LED"
                        }
                        button {
                            class: format!(
                                "relative inline-flex h-6 w-12 items-center rounded-full transition-colors {}",
                                if ir_enabled() { "bg-blue-500" } else { "bg-gray-600" }
                            ),
                            onclick: move |_| {
                                let new_state = !ir_enabled();
                                spawn(async move {
                                    if let Ok(state) = toggle_ir_led(new_state).await {
                                        ir_enabled.set(state);
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

                    div {
                        class: "flex items-center gap-3",
                        label {
                            class: format!(
                                "font-small whitespace-nowrap {}",
                                if is_admin_user() { "text-white" } else { "text-gray-500" }
                            ),
                            "IR Filter"
                        }
                        button {
                            class: format!(
                                "relative inline-flex h-6 w-12 items-center rounded-full transition-colors {} {}",
                                if ir_filter_enabled() {
                                    if is_admin_user() { "bg-blue-500" } else { "bg-gray-500" }
                                } else {
                                    "bg-gray-600"
                                },
                                if !is_admin_user() { "opacity-50 cursor-not-allowed" } else { "cursor-pointer" }
                            ),
                            disabled: !is_admin_user(),
                            onclick: move |_| {
                                if is_admin_user() {
                                    let new_state = !ir_filter_enabled();
                                    spawn(async move {
                                        if let Ok(state) = toggle_ir_filter(new_state).await {
                                            ir_filter_enabled.set(state);
                                        }
                                    });
                                }
                            },
                            span {
                                class: format!(
                                    "inline-block h-4 w-4 transform rounded-full bg-white transition-transform {}",
                                    if ir_filter_enabled() { "translate-x-7" } else { "translate-x-1" }
                                )
                            }
                        }
                    }
                }


                div {
                    class: "w-full flex flex-col items-center gap-6 px-4",
                    style: "--content-width: min(100%, 1280px); --stream-height: calc(var(--content-width) * 9 / 16); --spec-height: calc(var(--content-width) * 4 / 16);",
                    iframe {
                        src: stream_url,
                        style: format!(
                            "height: var(--stream-height); aspect-ratio: 16 / 9; width: var(--content-width); {}",
                             "" // if !ir_filter_enabled() { "filter: grayscale(100%);" } else { "" }
                        ),
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

            // Grafana panels grid
            div {
                class: "w-full max-w-7xl grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4",

                // Temperature panel
                    // <iframe src="http://localhost:3000/d-solo/adv7pb5/voegeli?orgId=1&timezone=browser&refresh=5s&panelId=panel-7&__feature.dashboardSceneSolo=true" width="450" height="200" frameborder="0"></iframe>
                iframe {
                    src: format!("{}/d-solo/{}?orgId=1&timezone=browser&refresh=5s&panelId=panel-7&__feature.dashboardSceneSolo=true&from=now-24h&to=now", cfg.grafana_base_url, cfg.grafana_dashboard),
                    class: "w-full h-64 rounded-lg border-2 border-slate-700",
                }

                // Humidity panel
                iframe {
                    src: format!("{}/d-solo/{}?orgId=1&timezone=browser&refresh=5s&panelId=panel-8&__feature.dashboardSceneSolo=true&from=now-24h&to=now", cfg.grafana_base_url, cfg.grafana_dashboard),
                    class: "w-full h-64 rounded-lg border-2 border-slate-700",
                }

                // CO2 panel
                iframe {
                    src: format!("{}/d-solo/{}?orgId=1&timezone=browser&refresh=5s&panelId=panel-9&__feature.dashboardSceneSolo=true&from=now-24h&to=now", cfg.grafana_base_url, cfg.grafana_dashboard),
                    class: "w-full h-64 rounded-lg border-2 border-slate-700",
                }

                // Motion panel
                iframe {
                    src: format!("{}/d-solo/{}?orgId=1&timezone=browser&refresh=5s&panelId=panel-10&__feature.dashboardSceneSolo=true&from=now-24h&to=now", cfg.grafana_base_url, cfg.grafana_dashboard),
                    class: "w-full h-64 rounded-lg border-2 border-slate-700",
                }

                // Visitors panel
                iframe {
                    src: format!("{}/d-solo/{}?orgId=1&timezone=browser&refresh=5s&panelId=panel-12&__feature.dashboardSceneSolo=true&from=now-24h&to=now", cfg.grafana_base_url, cfg.grafana_dashboard),
                    class: "w-full h-64 rounded-lg border-2 border-slate-700",
                }

                // Visitors panel
                iframe {
                    src: format!("{}/d-solo/{}?orgId=1&timezone=browser&refresh=5s&panelId=panel-11&__feature.dashboardSceneSolo=true&from=now-24h&to=now", cfg.grafana_base_url, cfg.grafana_dashboard),
                    class: "w-full h-64 rounded-lg border-2 border-slate-700",
                }
            }
            }
    }
}
