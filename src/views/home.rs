use dioxus::dioxus_core::Task;
use dioxus::document::eval;
use dioxus::prelude::*;

pub fn Home() -> Element {
    let mut _ws_task = use_signal(|| None::<Task>);

    use_effect(move || {
        let task = spawn(async move {

            eval(r#"

                const canvas = document.getElementById("spec");
                const gl = canvas.getContext("webgl", {
                    alpha: false,
                    antialias: false,
                    depth: false,
                    stencil: false,
                    preserveDrawingBuffer: true,  // KEY: Preserve buffer to avoid re-rendering
                    powerPreference: "low-power"
                });

                if (!gl) {
                    console.error("WebGL not supported");
                    return;
                }

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
                    void main() {
                        texCoord = position * 0.5 + 0.5;
                        texCoord.y = 1.0 - texCoord.y;
                        gl_Position = vec4(position, 0.0, 1.0);
                    }
                `);
                gl.compileShader(vertexShader);

                const fragmentShader = gl.createShader(gl.FRAGMENT_SHADER);
                gl.shaderSource(fragmentShader, `
                    precision mediump float;
                    uniform sampler2D tex;
                    uniform float offset;
                    varying vec2 texCoord;
                    void main() {
                        vec2 coord = texCoord;
                        coord.x = mod(coord.x + offset, 1.0);
                        gl_FragColor = texture2D(tex, coord);
                    }
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
                for (let y = 0; y < HEIGHT; y++) {
                    const frac = y / HEIGHT;
                    const freq = FREQ_MIN * Math.pow(FREQ_MAX / FREQ_MIN, frac);
                    const bin = Math.round(freq * FFT_SIZE / SAMPLE_RATE);
                    logFreqMap[HEIGHT - 1 - y] = Math.min(bin, FFT_SIZE / 2 - 1);
                }

                let lastRenderTime = 0;
                const MIN_FRAME_TIME = 1000 / FPS;

                const ws = new WebSocket('ws://127.0.0.1:8000/ws');
                ws.onopen = () => console.log("✅ WebSocket connected");
                ws.onerror = (err) => console.error("❌ WebSocket error:", err);

                ws.onmessage = (event) => {
                    const now = performance.now();
                    if (now - lastRenderTime < MIN_FRAME_TIME) {
                        return;  // Drop frames if receiving too fast
                    }
                    lastRenderTime = now;

                    const fftData = JSON.parse(event.data);

                    // Update texture
                    const column = new Uint8Array(HEIGHT * 4);
                    for (let y = 0; y < HEIGHT; y++) {
                        const v = fftData[logFreqMap[y]];
                        const norm = Math.max(0, Math.min(1, (v + 6) / 6));
                        const c = Math.floor(norm * 255);
                        column[y * 4] = c;
                        column[y * 4 + 1] = Math.floor(c * 0.6);
                        column[y * 4 + 2] = 255;
                        column[y * 4 + 3] = 255;
                    }

                    const x = Math.floor(scrollOffset * WIDTH);
                    gl.texSubImage2D(gl.TEXTURE_2D, 0, x, 0, 1, HEIGHT, gl.RGBA, gl.UNSIGNED_BYTE, column);

                    scrollOffset = (scrollOffset + 1 / WIDTH) % 1;

                    // Render ONLY when data arrives
                    gl.clearColor(0, 0, 0, 1);
                    gl.clear(gl.COLOR_BUFFER_BIT);
                    gl.uniform1f(offsetLoc, scrollOffset);
                    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
                };

                // Initial render
                gl.clearColor(0, 0, 0, 1);
                gl.clear(gl.COLOR_BUFFER_BIT);
                gl.uniform1f(offsetLoc, 0);
                gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
            "#);
        });
        _ws_task.set(Some(task));
    });

    rsx! {
        section {
            class: "min-h-screen w-full flex flex-col items-center justify-center bg-slate-900 gap-4",
            iframe {
                src: "http://localhost:8889/cam",
                class: "w-full max-w-4xl rounded-lg bg-gray-800 h-96",
                allow: "camera;microphone;autoplay",
            }
            canvas {
                id: "spec",
                class: "w-full max-w-4xl h-64 bg-black rounded-lg",
            }
        }
    }
}