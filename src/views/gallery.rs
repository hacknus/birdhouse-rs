use std::ops::Deref;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct ImageInfo {
    filename: String,
    url: String,
}

pub fn Gallery() -> Element {
    let mut images = use_signal(|| Vec::<ImageInfo>::new());
    let mut selected_image = use_signal(|| None::<usize>);
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);

    // Fetch images on component mount
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
                // Image grid
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
                    // Full image view
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

    // Handle keyboard events
    let handle_keydown = move |evt: KeyboardEvent| {
        match evt.key().to_string().as_str() {
            "Escape" => on_close.call(()),
            "ArrowLeft" => on_prev.call(()),
            "ArrowRight" => on_next.call(()),
            _ => {}
        }
    };

    // Save scroll position and prevent body scroll
    // Save scroll position and prevent body scroll
    use_effect(move || {
        spawn(async move {
            if let Some(window) = web_sys::window() {
                // Save current scroll position
                let scroll_y = window.scroll_y().unwrap_or(0.0);
                saved_scroll_y.set(scroll_y);

                if let Some(document) = window.document() {
                    // Focus the viewer
                    if let Some(element) = document.query_selector("[data-viewer]").ok().flatten() {
                        if let Some(html_element) = element.dyn_ref::<web_sys::HtmlElement>() {
                            let _ = html_element.focus();
                        }
                    }

                    // Prevent body scroll while maintaining position
                    if let Some(body) = document.body() {
                        let style = format!("overflow: hidden; position: fixed; width: 100%; top: -{}px;", scroll_y);
                        let _ = body.set_attribute("style", &style);
                    }
                }
            }
        });
    });

    // Restore scroll position on unmount
    use_drop(move || {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(body) = document.body() {
                    let _ = body.remove_attribute("style");
                    // Force scroll restoration
                    let scroll_y = saved_scroll_y();
                    let _ = window.scroll_to_with_x_and_y(0.0, scroll_y);
                }
            }
        }
    });


    rsx! {
        div {
            class: "fixed inset-0 bg-black bg-opacity-90 z-50 flex items-center justify-center",
            tabindex: 0,
            onkeydown: handle_keydown,
            "data-viewer": "true",

            // Close button
            button {
                class: "absolute top-4 right-4 text-white text-3xl hover:text-red-500 transition-colors z-10",
                onclick: move |_| on_close.call(()),
                "✕"
            }

            // Previous button
            button {
                class: "absolute left-4 text-white text-5xl hover:text-blue-500 transition-colors",
                onclick: move |_| on_prev.call(()),
                "‹"
            }

            // Image container
            div { class: "max-w-7xl max-h-screen p-8 flex flex-col items-center",
                img {
                    src: "{current_image.url}",
                    alt: "{current_image.filename}",
                    class: "max-w-full max-h-[80vh] object-contain rounded-lg"
                }
                p { class: "text-white mt-4 text-lg",
                    "{current_image.filename} ({idx + 1} of {images.len()})"
                }
            }

            // Next button
            button {
                class: "absolute right-4 text-white text-5xl hover:text-blue-500 transition-colors",
                onclick: move |_| on_next.call(()),
                "›"
            }
        }
    }
}




// Server function to fetch images
#[server]
async fn fetch_images() -> Result<Vec<ImageInfo>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use std::fs;
        use std::path::Path;

        let local_cache = "./public/gallery_cache";

        // Create directory if it doesn't exist
        fs::create_dir_all(local_cache)
            .map_err(|e| ServerFnError::new(format!("Failed to create cache dir: {}", e)))?;

        let entries = fs::read_dir(local_cache)
            .map_err(|e| ServerFnError::new(format!("Failed to read directory: {}", e)))?;

        let mut images = Vec::new();

        for entry in entries {
            let entry = entry.map_err(|e| ServerFnError::new(format!("Failed to read entry: {}", e)))?;
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



