use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::views::gallery_client::{lock_scroll, save_scroll_position, unlock_scroll};

const ARROW_LEFT: Asset = asset!("/assets/svg/arrow-left-svgrepo-com.svg");
const ARROW_RIGHT: Asset = asset!("/assets/svg/arrow-right-svgrepo-com.svg");

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct ImageInfo {
    filename: String,
    url: String,
}

#[component]
fn ImageViewer(
    images: Vec<ImageInfo>,
    current_index: Option<usize>,
    on_close: EventHandler<()>,
    on_next: EventHandler<()>,
    on_prev: EventHandler<()>,
) -> Element {
    let Some(idx) = current_index else {
        return rsx! { div {} };
    };

    let current_image = &images[idx];
    let mut saved_scroll_y = use_signal(|| 0.0);

    // State to track swipe gestures
    let mut touch_start_x = use_signal(|| 0.0);
    let mut touch_current_x = use_signal(|| 0.0);

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

    // Capture the starting point of the touch
    let handle_touchstart = move |evt: TouchEvent| {
        if let Some(touch) = evt.touches().first() {
            let x = touch.page_coordinates().x as f64;
            touch_start_x.set(x);
            touch_current_x.set(x); // Initialize "current" to start
        }
    };

    // Continuously update the "current" point as the finger moves
    let handle_touchmove = move |evt: TouchEvent| {
        if let Some(touch) = evt.touches().first() {
            let x = touch.page_coordinates().x as f64;
            touch_current_x.set(x);
        }
    };

    // Calculate the difference between start and the last known move position
    let handle_touchend = move |evt: TouchEvent| {
        evt.stop_propagation(); // Try to stop click events firing after swipe

        let diff_x = touch_current_x() - touch_start_x();

        // 50px threshold for a swipe
        if diff_x.abs() > 50.0 {
            if diff_x > 0.0 {
                // Swiped Right -> Previous
                on_prev.call(());
            } else {
                // Swiped Left -> Next
                on_next.call(());
            }
        }
    };

    let handle_keydown = move |evt: KeyboardEvent| match evt.key().to_string().as_str() {
        "Escape" => on_close.call(()),
        "ArrowLeft" => on_prev.call(()),
        "ArrowRight" => on_next.call(()),
        _ => {}
    };

    rsx! {
        div {
            class: "backdrop fixed inset-0 bg-black bg-opacity-90 z-50 flex items-center justify-center",
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

            div {
                class: "viewer-content max-w-[90vw] max-h-[90vh]",
                onclick: move |evt: MouseEvent| evt.stop_propagation(),
                img {
                    src: current_image.url.clone(),
                    alt: current_image.filename.clone(),
                    class: "max-w-full max-h-full object-contain rounded-lg",
                }
            }
        }
    }
}

pub fn Gallery() -> Element {
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
                                    alt: "{img.filename}",
                                    class: "w-full h-64 object-cover group-hover:scale-105 transition-transform duration-200"
                                }
                                div { class: "absolute bottom-0 left-0 right-0 bg-black bg-opacity-50 p-2",
                                    p { class: "text-white text-sm truncate", "{img.filename}" }
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

        let local_cache = "./public/gallery_cache";

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
                            url: format!("/gallery_cache/{}", filename_str),
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
