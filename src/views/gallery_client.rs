#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(target_arch = "wasm32")]
pub fn save_scroll_position() -> f64 {
    if let Some(window) = web_sys::window() {
        window.scroll_y().unwrap_or(0.0)
    } else {
        0.0
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_scroll_position() -> f64 {
    0.0
}

#[cfg(target_arch = "wasm32")]
pub fn lock_scroll(scroll_y: f64) {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(element) = document.query_selector("[data-viewer]").ok().flatten() {
                if let Some(html_element) = element.dyn_ref::<web_sys::HtmlElement>() {
                    let _ = html_element.focus();
                }
            }

            if let Some(body) = document.body() {
                let style = format!(
                    "overflow: hidden; position: fixed; width: 100%; top: -{}px;",
                    scroll_y
                );
                let _ = body.set_attribute("style", &style);
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn lock_scroll(_scroll_y: f64) {}

#[cfg(target_arch = "wasm32")]
pub fn unlock_scroll(scroll_y: f64) {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(body) = document.body() {
                let _ = body.remove_attribute("style");
                let _ = window.scroll_to_with_x_and_y(0.0, scroll_y);
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn unlock_scroll(_scroll_y: f64) {}
