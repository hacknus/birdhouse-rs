// File: `src/views/gallery.rs`
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::views::gallery_client::{lock_scroll, save_scroll_position, unlock_scroll};
use std::path::Path;

const ARROW_LEFT: Asset = asset!("/assets/svg/arrow-left-svgrepo-com.svg");
const ARROW_RIGHT: Asset = asset!("/assets/svg/arrow-right-svgrepo-com.svg");

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct ImageInfo {
    filename: String,
    url: String,
}

// parse filename -> display string (tries to decode YYYYMMDD[ _- ]HHMMSS formats)
fn format_display(filename: &str) -> String {
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename)
        .trim()
        .to_string();

    if stem.is_empty() {
        return filename.to_string();
    }

    // normalize `YYYYMMDD_HHMMSS` or `YYYYMMDD-HHMMSS` -> `YYYYMMDDHHMMSS`
    let normalized = if stem.len() == 15 && (stem.chars().nth(8) == Some('_') || stem.chars().nth(8) == Some('-')) {
        let mut t = stem.clone();
        t.remove(8);
        t
    } else {
        stem.clone()
    };

    if normalized.len() == 14 && normalized.chars().all(|c| c.is_ascii_digit()) {
        let year = &normalized[0..4];
        let month = &normalized[4..6];
        let day = &normalized[6..8];
        let hour = &normalized[8..10];
        let minute = &normalized[10..12];
        let second = &normalized[12..14];
        format!("{}.{}.{} {}:{}:{}", day, month, year, hour, minute, second)
    } else {
        // fallback to original filename (including extension)
        filename.to_string()
    }
}

#[component]
fn ImageViewer(
    images: Vec<ImageInfo>,
    current_index: Option<usize>,
    on_close: EventHandler<()>,
    on_next: EventHandler<()>,
    on_prev: EventHandler<()>,
) -> Element {
    use std::time::Duration;
    use dioxus::prelude::spawn;

    let Some(idx) = current_index else {
        return rsx! { div {} };
    };

    let current_image = &images[idx];
    let current_display = format_display(&current_image.filename);
    let mut saved_scroll_y = use_signal(|| 0.0);

    // State to track swipe gestures
    let mut touch_start_x = use_signal(|| 0.0);
    let mut touch_current_x = use_signal(|| 0.0);

    // Visual offset (px) while dragging / animating
    let mut swipe_offset = use_signal(|| 0.0);
    // Whether transitions should be enabled (used to animate snap/slide)
    let mut is_animating = use_signal(|| false);

    use_effect(move || {
        spawn(async move {
            let scroll_y = save_scroll_position();
            saved_scroll_y.set(scroll_y);
            lock_scroll(scroll_y);
        });
    });

    use_drop(move || {
        unlock_scroll(saved_scroll_y());
    });

    // helper indices for prev/next with wrap
    let len = images.len();
    let prev_index = if len > 0 { (idx + len - 1) % len } else { idx };
    let next_index = if len > 0 { (idx + 1) % len } else { idx };

    // Capture the starting point of the touch
    let handle_touchstart = move |evt: TouchEvent| {
        if let Some(touch) = evt.touches().first() {
            let x = touch.page_coordinates().x as f64;
            touch_start_x.set(x);
            touch_current_x.set(x); // Initialize "current" to start
            // while dragging, disable CSS transition so strip follows finger
            is_animating.set(false);
            swipe_offset.set(0.0);
        }
    };

    // Continuously update the "current" point as the finger moves
    let handle_touchmove = move |evt: TouchEvent| {
        if let Some(touch) = evt.touches().first() {
            let x = touch.page_coordinates().x as f64;
            touch_current_x.set(x);
            let diff = touch_current_x() - touch_start_x();
            swipe_offset.set(diff);
        }
    };

    // Calculate the difference between start and the last known move position
    let handle_touchend = move |evt: TouchEvent| {
        evt.stop_propagation(); // Try to stop click events firing after swipe

        let diff_x = touch_current_x() - touch_start_x();
        let threshold = 50.0;

        // If not compiled to wasm, just call handlers immediately (no animation)
        #[cfg(not(target_arch = "wasm32"))]
        {
            if diff_x.abs() > threshold {
                if diff_x > 0.0 {
                    on_prev.call(());
                } else {
                    on_next.call(());
                }
            }
            swipe_offset.set(0.0);
            return;
        }

        // wasm-only animated behaviour
        #[cfg(target_arch = "wasm32")]
        {
            // decide swipe or snap back
            if diff_x.abs() > threshold {
                // animate strip so the neighbouring slide becomes centered
                is_animating.set(true);

                // compute viewport width in pixels
                let viewport_px = web_sys::window()
                    .and_then(|w| w.inner_width().ok())
                    .and_then(|v| v.as_f64())
                    .or_else(|| {
                        web_sys::window()
                            .and_then(|w| w.document())
                            .and_then(|d| d.document_element())
                            .map(|el| el.client_width() as f64)
                    })
                    .unwrap_or(0.0);

                // target offset: +/- viewport width so transform goes to 0vw (prev) or -200vw (next)
                let end_px = if diff_x > 0.0 { viewport_px } else { -viewport_px };
                swipe_offset.set(end_px);

                let on_next = on_next.clone();
                let on_prev = on_prev.clone();
                let mut swipe_offset_clone = swipe_offset.clone();
                let mut is_animating_clone = is_animating.clone();
                let direction_positive = diff_x > 0.0;

                spawn(async move {
                    // wait for CSS transition (match duration below)
                    gloo_timers::future::sleep(Duration::from_millis(300)).await;
                    if direction_positive {
                        on_prev.call(());
                    } else {
                        on_next.call(());
                    }
                    // reset position without transition (prepares for next image)
                    swipe_offset_clone.set(0.0);
                    is_animating_clone.set(false);
                });
            } else {
                // not a swipe -> snap back to center
                is_animating.set(true);
                swipe_offset.set(0.0);

                let mut is_animating_clone = is_animating.clone();
                spawn(async move {
                    gloo_timers::future::sleep(Duration::from_millis(180)).await;
                    is_animating_clone.set(false);
                });
            }
        }
    };

    let handle_keydown = move |evt: KeyboardEvent| match evt.key().to_string().as_str() {
        "Escape" => on_close.call(()),
        "ArrowLeft" => on_prev.call(()),
        "ArrowRight" => on_next.call(()),
        _ => {}
    };

    // Small horizontal gap between images (total gap in px)
    let gap_px = 32; // small distance between images while swiping
    let half_gap = gap_px / 2;

    // Build transform using *vw* units derived from the current px offset so px/vw mixing can't desync directions.
    let style_string = {
        let offset_px = swipe_offset();
        let transition = if is_animating() { "transform 300ms ease" } else { "none" };

        // viewport width in pixels (wasm only). Non-wasm fallback to 1.0 to avoid divide-by-zero.
        #[cfg(target_arch = "wasm32")]
        let viewport_px = web_sys::window()
            .and_then(|w| w.inner_width().ok())
            .and_then(|v| v.as_f64())
            .or_else(|| {
                web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.document_element())
                    .map(|el| el.client_width() as f64)
            })
            .unwrap_or(1.0);

        #[cfg(not(target_arch = "wasm32"))]
        let viewport_px: f64 = 1.0;

        let offset_vw = if viewport_px.abs() > 0.0 {
            (offset_px / viewport_px) * 100.0
        } else {
            0.0
        };

        // center at -100vw, add the offset in vw so final targets become 0vw or -200vw exactly when offset_vw is ±100
        format!(
            "width: 300vw; display: flex; align-items: center; justify-content: flex-start; transform: translateX(calc(-100vw + {}vw)); transition: {};",
            offset_vw, transition
        )
    };

    // each slide equals viewport width so adjacent slides sit exactly next to the current
    let slide_style = "flex: 0 0 100vw; display: flex; align-items: center; justify-content: center;";

    // inner slide wrapper pads left/right so images don't touch edges and create a small gap between slides
    let inner_slide_style = format!("width: 100%; padding: 0 {}px; display: flex; align-items: center; justify-content: center;", half_gap);

    // Ensure image leaves room for navbar (52px) and caption (~120px)
    let img_style = format!(
        "width: calc(100vw - {}px); max-height: calc(100vh - 52px - 120px); object-fit: contain; border-radius: 8px;",
        gap_px
    );

    let prev_display = format_display(&images[prev_index].filename);
    let next_display = format_display(&images[next_index].filename);

    rsx! {
        // Position viewer under navbar by setting top to 52px so navbar top remains visible.
        div {
            class: "backdrop fixed bg-black bg-opacity-90 z-50 flex items-center justify-center",
            style: "top: 52px; left: 0; right: 0; bottom: 0;",
            tabindex: 0,
            "data-viewer": "true",
            onkeydown: handle_keydown,
            ontouchstart: handle_touchstart,
            ontouchmove: handle_touchmove,
            ontouchend: handle_touchend,
            onclick: move |_evt: MouseEvent| on_close.call(()),
            onmounted: move |_| {
                #[cfg(target_arch = "wasm32")]
                {
                    if let Some(window) = web_sys::window() {
                        if let Some(document) = window.document() {
                            if let Some(element) = document.query_selector("[data-viewer]").ok().flatten() {
                                use wasm_bindgen::JsCast;
                                if let Some(html_element) = element.dyn_ref::<web_sys::HtmlElement>() {
                                    let _ = html_element.focus();
                                }
                            }
                        }
                    }
                }
            },

            button {
                class: "absolute right-4 text-white text-3xl hover:text-red-500 transition-colors z-60 bg-black bg-opacity-30 rounded-lg px-3 py-2",
                style: "top: 68px;",
                onclick: move |evt| {
                    evt.stop_propagation();
                    on_close.call(());
                },
                "✕"
            }

            button {
                class: "absolute left-4 top-1/2 -translate-y-1/2 z-10 bg-white bg-opacity-30 rounded-lg px-3 py-2 h-12 w-auto flex items-center justify-center hover:bg-opacity-50 transition-all",
                onclick: move |evt| {
                    evt.stop_propagation();
                    on_prev.call(());
                },
                img {
                    src: ARROW_LEFT,
                    class: "w-6 h-6",
                    alt: "Previous"
                }
            }

            button {
                class: "absolute right-4 top-1/2 -translate-y-1/2 z-10 bg-white bg-opacity-30 rounded-lg px-3 py-2 h-12 w-auto flex items-center justify-center hover:bg-opacity-50 transition-all",
                onclick: move |evt| {
                    evt.stop_propagation();
                    on_next.call(());
                },
                img {
                    src: ARROW_RIGHT,
                    class: "w-6 h-6",
                    alt: "Next"
                }
            }

            // viewer container uses available height under navbar
            div {
                class: "viewer-content w-screen max-h-[90vh] overflow-hidden relative",
                onclick: move |evt: MouseEvent| evt.stop_propagation(),
                // strip that will be translated during swipe (contains prev, current, next)
                div {
                    style: "{style_string}",
                    // prev slide
                    div { style: "{slide_style}",
                        div { style: "{inner_slide_style}",
                            img {
                                style: "{img_style}",
                                src: images[prev_index].url.clone(),
                                alt: "{prev_display}",
                                class: "rounded-lg",
                            }
                        }
                    }
                    // current slide
                    div { style: "{slide_style}",
                        div { style: "{inner_slide_style}",
                            img {
                                style: "{img_style}",
                                src: current_image.url.clone(),
                                alt: "{current_display}",
                                class: "rounded-lg",
                            }
                        }
                    }
                    // next slide
                    div { style: "{slide_style}",
                        div { style: "{inner_slide_style}",
                            img {
                                style: "{img_style}",
                                src: images[next_index].url.clone(),
                                alt: "{next_display}",
                                class: "rounded-lg",
                            }
                        }
                    }
                }
                // Caption underneath the image showing parsed date/time (fallbacks to filename)
                div {
                    class: "mt-4 text-center text-white text-sm select-none",
                    p { class: "opacity-90", "{current_display}" }
                }
            }
        }
    }
}

// rust
// File: `src/views/gallery.rs` — replaced `Gallery` function
pub fn Gallery() -> Element {
    use dioxus::prelude::spawn;

    let mut images = use_signal(|| Vec::<ImageInfo>::new());
    let mut selected_image = use_signal(|| None::<usize>);
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);

    use_effect(move || {
        spawn(async move {
            match fetch_images().await {
                Ok(imgs) => {
                    images.set(imgs);
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    rsx! {
        div { class: "min-h-screen w-full bg-slate-900 p-8",
            h1 { class: "text-4xl font-bold text-white mb-8 text-center", "gallery" }
            if loading() {
                div { class: "flex justify-center items-center h-64",
                    p { class: "text-white text-xl", "Loading images..." }
                }
            } else if let Some(err) = error() {
                div { class: "flex justify-center items-center h-64",
                    p { class: "text-red-500 text-xl", "Error: {err}" }
                }
            } else if images().is_empty() {
                div { class: "flex justify-center items-center h-64",
                    p { class: "text-white text-xl", "No images found" }
                }
            } else {
                if selected_image().is_none() {
                    div { class: "grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4",
                        for (idx, img) in images().iter().enumerate() {
                            div {
                                key: "{img.filename}",
                                class: "relative group cursor-pointer overflow-hidden rounded-lg bg-slate-800 hover:ring-2 hover:ring-blue-500 transition-all",
                                onclick: move |_| selected_image.set(Some(idx)),
                                img {
                                    src: "{img.url}",
                                    // call format_display inline to avoid `let` inside rsx!
                                    alt: "{format_display(&img.filename)}",
                                    class: "w-full h-64 object-cover group-hover:scale-105 transition-transform duration-200"
                                }
                                div { class: "absolute bottom-0 left-0 right-0 bg-black bg-opacity-50 p-2",
                                    p { class: "text-white text-sm truncate", "{format_display(&img.filename)}" }
                                }
                            }
                        }
                    }
                } else {
                    ImageViewer {
                        images: images(),
                        current_index: selected_image(),
                        on_close: move |_| selected_image.set(None),
                        on_next: move |_| {
                            if let Some(idx) = selected_image() {
                                let next = (idx + 1) % images().len();
                                selected_image.set(Some(next));
                            }
                        },
                        on_prev: move |_| {
                            if let Some(idx) = selected_image() {
                                let prev = if idx == 0 { images().len() - 1 } else { idx - 1 };
                                selected_image.set(Some(prev));
                            }
                        }
                    }
                }
            }
        }
    }
}

#[server]
async fn fetch_images() -> Result<Vec<ImageInfo>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use std::fs;

        let local_cache = "./gallery";

        fs::create_dir_all(local_cache)
            .map_err(|e| ServerFnError::new(format!("Failed to create cache dir: {}", e)))?;

        let entries = fs::read_dir(local_cache)
            .map_err(|e| ServerFnError::new(format!("Failed to read directory: {}", e)))?;

        let mut images = Vec::new();

        for entry in entries {
            let entry =
                entry.map_err(|e| ServerFnError::new(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();

            if let Some(ext) = path.extension() {
                let ext = ext.to_string_lossy().to_lowercase();
                if matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "gif" | "webp") {
                    if let Some(filename) = path.file_name() {
                        let filename_str = filename.to_string_lossy().to_string();
                        images.push(ImageInfo {
                            url: format!("/gallery/{}", filename_str),
                            filename: filename_str,
                        });
                    }
                }
            }
        }

        images.sort_by(|a, b| b.filename.cmp(&a.filename));

        Ok(images)
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(Vec::new())
    }
}