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

    match tcp_client::send_command(cmd) {
        Ok(_) => Ok(enabled),
        Err(e) => Err(ServerFnError::new(e)),
    }
}

#[server]
async fn get_ir_state() -> Result<bool, ServerFnError> {
    match tcp_client::send_command("[CMD] GET IR STATE") {
        Ok(response) => {
            let is_on = response.to_lowercase().contains("on") || response.contains("1");
            Ok(is_on)
        }
        Err(e) => Err(ServerFnError::new(e)),
    }
}

#[server]
async fn is_admin() -> Result<bool, ServerFnError> {
    // TODO: Implement actual admin check (e.g., check session, JWT token, etc.)
    // For now, return false
    Ok(false)
}

#[server]
async fn toggle_ir_filter(enabled: bool) -> Result<bool, ServerFnError> {
    let cmd = if enabled {
        "[CMD] IR FILTER ON"
    } else {
        "[CMD] IR FILTER OFF"
    };

    match tcp_client::send_command(cmd) {
        Ok(_) => Ok(enabled),
        Err(e) => Err(ServerFnError::new(e)),
    }
}

#[server]
async fn get_admin_feature_state() -> Result<bool, ServerFnError> {
    match tcp_client::send_command("[CMD] GET IR FILTER STATE") {
        Ok(response) => {
            let is_on = response.to_lowercase().contains("on") || response.contains("1");
            Ok(is_on)
        }
        Err(e) => Err(ServerFnError::new(e)),
    }
}

pub fn Home() -> Element {
    let mut config = use_resource(|| async move { get_stream_config().await.ok() });
    let mut _ws_task = use_signal(|| None::<Task>);
    let tcp_ws_task = use_signal(|| None::<Task>);

    let mut tcp_state = use_context::<tcp_state::TcpState>();
    let mut ir_enabled = tcp_state.ir_enabled;
    let mut ir_filter_enabled = tcp_state.ir_filter_enabled;
    let mut is_admin_user = tcp_state.is_admin;

    // Track if initial states are loaded
    let mut initial_states_loaded = use_signal(|| false);

    // Load initial states from server
    let ir_state_resource = use_resource(move || async move {
        get_ir_state().await.ok()
    });

    let ir_filter_resource = use_resource(move || async move {
        get_admin_feature_state().await.ok()
    });

    let admin_resource = use_resource(move || async move {
        is_admin().await.ok()
    });

    // Update signals when all resources are ready
    use_effect(move || {
        let ir_ready = ir_state_resource.read().is_some();
        let filter_ready = ir_filter_resource.read().is_some();
        let admin_ready = admin_resource.read().is_some();

        if ir_ready && filter_ready && admin_ready && !initial_states_loaded() {
            if let Some(Some(state)) = ir_state_resource.read().as_ref() {
                ir_enabled.set(*state);
            }
            if let Some(Some(state)) = ir_filter_resource.read().as_ref() {
                ir_filter_enabled.set(*state);
            }
            if let Some(Some(admin)) = admin_resource.read().as_ref() {
                is_admin_user.set(*admin);
            }
            initial_states_loaded.set(true);
        }
    });

    // Only initialize WebSocket after initial states are loaded
    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        if initial_states_loaded() && tcp_state.ws_initialized.read().is_none() {
            tcp_state.init_websocket();
        }
    });

    // Check admin status on mount
    use_resource(move || async move {
        if let Ok(admin) = is_admin().await {
            is_admin_user.set(admin);
        }
    });

    let config_value = config.read();
    let Some(Some(cfg)) = config_value.as_ref() else {
        return rsx! { div { "Loading..." } };
    };

    let stream_url = cfg.stream_url.clone();
    let ws_url = cfg.websocket_url.clone();

    #[cfg(target_arch = "wasm32")]
    let tcp_ws_handle = use_signal(|| None::<WebSocket>);

    #[cfg(target_arch = "wasm32")]
    {
        let mut ws_store = tcp_ws_handle.clone();
        let mut ir_enabled_signal = ir_enabled.clone();
        let mut ir_filter_signal = ir_filter_enabled.clone();

        use_effect(move || {
            if ws_store.read().is_some() {
                return;
            }

            let window = web_sys::window().expect("browser window");
            let host = window
                .location()
                .host()
                .unwrap_or_else(|_| "127.0.0.1:8080".into());
            let socket =
                WebSocket::new(&format!("ws://{host}/ws/tcp")).expect("open TCP websocket");

            let on_message = {
                let mut ir_enabled_signal = ir_enabled_signal.clone();
                let mut ir_filter_signal = ir_filter_signal.clone();
                Closure::wrap(Box::new(move |event: MessageEvent| {
                    if let Some(text) = event.data().as_string() {
                        let payload = text.to_uppercase();
                        match () {
                            _ if payload.contains("IR LED STATE: ON")
                                || payload.contains("IR STATE IS ON")
                                || payload.contains("IR ON") =>
                            {
                                ir_enabled_signal.set(true);
                            }
                            _ if payload.contains("IR LED STATE: OFF")
                                || payload.contains("IR STATE IS OFF")
                                || payload.contains("IR OFF") =>
                            {
                                ir_enabled_signal.set(false);
                            }
                            _ if payload.contains("IR FILTER STATE: ON")
                                || payload.contains("IR FILTER STATE IS ON")
                                || payload.contains("IR FILTER ON") =>
                            {
                                ir_filter_signal.set(true);
                            }
                            _ if payload.contains("IR FILTER STATE: OFF")
                                || payload.contains("IR FILTER STATE IS OFF")
                                || payload.contains("IR FILTER OFF") =>
                            {
                                ir_filter_signal.set(false);
                            }
                            _ => {}
                        }
                    }
                }) as Box<dyn FnMut(_)>)
            };

            socket.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
            on_message.forget();

            ws_store.set(Some(socket));
        });
    }

    use_effect(move || {
        let ws_url = ws_url.clone();
        let task = spawn(async move {
            let script = format!(
                r#"
                const canvas = document.getElementById("spec");
                const gl = canvas.getContext("webgl", {{
                    alpha: false,
                    antialias: false,
                    depth: false,
                    stencil: false,
                    preserveDrawingBuffer: true,
                    powerPreference: "low-power"
                }});

                if (!gl) {{
                    console.error("WebGL not supported");
                    return;
                }}

                const HEIGHT = 256;
                const SECONDS = 15;
                const FPS = 30;
                const WIDTH = SECONDS * FPS;

                canvas.width = WIDTH;
                canvas.height = HEIGHT;
                gl.viewport(0, 0, WIDTH, HEIGHT);

                const texture = gl.createTexture();
                gl.bindTexture(gl.TEXTURE_2D, texture);
                gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
                gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
                gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
                gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);

                const pixels = new Uint8Array(WIDTH * HEIGHT * 4);
                gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, WIDTH, HEIGHT, 0, gl.RGBA, gl.UNSIGNED_BYTE, pixels);

                const vertexShader = gl.createShader(gl.VERTEX_SHADER);
                gl.shaderSource(vertexShader, `
                    attribute vec2 position;
                    varying vec2 texCoord;
                    void main() {{
                        texCoord = position * 0.5 + 0.5;
                        texCoord.y = 1.0 - texCoord.y;
                        gl_Position = vec4(position, 0.0, 1.0);
                    }}
                `);
                gl.compileShader(vertexShader);

                const fragmentShader = gl.createShader(gl.FRAGMENT_SHADER);
                gl.shaderSource(fragmentShader, `
                    precision mediump float;
                    uniform sampler2D tex;
                    uniform float offset;
                    varying vec2 texCoord;
                    void main() {{
                        vec2 coord = texCoord;
                        coord.x = mod(coord.x + offset, 1.0);
                        gl_FragColor = texture2D(tex, coord);
                    }}
                `);
                gl.compileShader(fragmentShader);

                const program = gl.createProgram();
                gl.attachShader(program, vertexShader);
                gl.attachShader(program, fragmentShader);
                gl.linkProgram(program);
                gl.useProgram(program);

                const positionBuffer = gl.createBuffer();
                gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
                gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1,-1, 1,-1, -1,1, 1,1]), gl.STATIC_DRAW);

                const positionLoc = gl.getAttribLocation(program, "position");
                gl.enableVertexAttribArray(positionLoc);
                gl.vertexAttribPointer(positionLoc, 2, gl.FLOAT, false, 0, 0);

                const offsetLoc = gl.getUniformLocation(program, "offset");
                let scrollOffset = 0;

                const FREQ_MIN = 200;
                const FREQ_MAX = 12000;
                const SAMPLE_RATE = 44100;
                const FFT_SIZE = 1024;

                const logFreqMap = new Array(HEIGHT);
                for (let y = 0; y < HEIGHT; y++) {{
                    const frac = y / HEIGHT;
                    const freq = FREQ_MIN * Math.pow(FREQ_MAX / FREQ_MIN, frac);
                    const bin = Math.round(freq * FFT_SIZE / SAMPLE_RATE);
                    logFreqMap[HEIGHT - 1 - y] = Math.min(bin, FFT_SIZE / 2 - 1);
                }}

                let lastRenderTime = 0;
                const MIN_FRAME_TIME = 1000 / FPS;

                const ws = new WebSocket('{ws_url}');
                ws.onopen = () => console.log("WebSocket connected");
                ws.onerror = (err) => console.error("WebSocket error:", err);

                ws.onmessage = (event) => {{
                    const now = performance.now();
                    if (now - lastRenderTime < MIN_FRAME_TIME) {{
                        return;
                    }}
                    lastRenderTime = now;

                    const fftData = JSON.parse(event.data);

                    const column = new Uint8Array(HEIGHT * 4);
                    for (let y = 0; y < HEIGHT; y++) {{
                        const v = fftData[logFreqMap[y]];
                        const norm = Math.max(0, Math.min(1, (v + 6) / 6));
                        const c = Math.floor(norm * 255);
                        column[y * 4] = c;
                        column[y * 4 + 1] = Math.floor(c * 0.6);
                        column[y * 4 + 2] = 255;
                        column[y * 4 + 3] = 255;
                    }}

                    const x = Math.floor(scrollOffset * WIDTH);
                    gl.texSubImage2D(gl.TEXTURE_2D, 0, x, 0, 1, HEIGHT, gl.RGBA, gl.UNSIGNED_BYTE, column);

                    scrollOffset = (scrollOffset + 1 / WIDTH) % 1;

                    gl.clearColor(0, 0, 0, 1);
                    gl.clear(gl.COLOR_BUFFER_BIT);
                    gl.uniform1f(offsetLoc, scrollOffset);
                    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
                }};

                gl.clearColor(0, 0, 0, 1);
                gl.clear(gl.COLOR_BUFFER_BIT);
                gl.uniform1f(offsetLoc, 0);
                gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
            "#
            );

            eval(&script);
        });
        _ws_task.set(Some(task));
    });

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
                            style: "height: var(--stream-height); aspect-ratio: 16 / 9; width: var(--content-width);",
                            class: "rounded-lg bg-gray-800 shadow-lg",
                            allow: "camera;autoplay",
                        }
                        canvas {
                            id: "spec",
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
                        src: format!("{}/d-solo/{}?orgId=1&timezone=browser&refresh=5s&panelId=panel-7&__feature.dashboardSceneSolo=true&from=now-6h&to=now", cfg.grafana_base_url, cfg.grafana_dashboard),
                        class: "w-full h-64 rounded-lg border-2 border-slate-700",
                    }

                    // Humidity panel
                    iframe {
                        src: format!("{}/d-solo/{}?orgId=1&timezone=browser&refresh=5s&panelId=panel-8&__feature.dashboardSceneSolo=true&from=now-6h&to=now", cfg.grafana_base_url, cfg.grafana_dashboard),
                        class: "w-full h-64 rounded-lg border-2 border-slate-700",
                    }

                    // CO2 panel
                    iframe {
                        src: format!("{}/d-solo/{}?orgId=1&timezone=browser&refresh=5s&panelId=panel-9&__feature.dashboardSceneSolo=true&from=now-6h&to=now", cfg.grafana_base_url, cfg.grafana_dashboard),
                        class: "w-full h-64 rounded-lg border-2 border-slate-700",
                    }

                    // Motion panel
                    iframe {
                        src: format!("{}/d-solo/{}?orgId=1&timezone=browser&refresh=5s&panelId=panel-10&__feature.dashboardSceneSolo=true&from=now-6h&to=now", cfg.grafana_base_url, cfg.grafana_dashboard),
                        class: "w-full h-64 rounded-lg border-2 border-slate-700",
                    }

                    // Visitors panel
                    iframe {
                        src: format!("{}/d-solo/{}?orgId=1&timezone=browser&refresh=5s&panelId=panel-11&__feature.dashboardSceneSolo=true&from=now-6h&to=now", cfg.grafana_base_url, cfg.grafana_dashboard),
                        class: "w-full h-64 rounded-lg border-2 border-slate-700",
                    }
                }
                }
        }
}
