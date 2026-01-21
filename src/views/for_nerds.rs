use crate::Route;
use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use js_sys::eval;

const MAP_CSS: Asset = asset!("/assets/leaflet/leaflet.css",);
const MAP_JS: Asset = asset!("/assets/leaflet/leaflet.js",);
const MAP_INIT_JS: Asset = asset!("/assets/js/map_init.js",);

#[component]
pub fn ForNerds() -> Element {

    use_effect(|| {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = eval("window.initLeafletMap && window.initLeafletMap();");
        }
    });

    rsx! {
        document::Link { rel: "stylesheet", href: MAP_CSS }
        document::Script { src: MAP_JS }
        document::Script { src: MAP_INIT_JS }

       div {
            class: "w-full flex flex-col items-center gap-6 px-4",
            style: "--content-width: min(100%, 1280px); --map-height: calc(var(--content-width) * 9 / 16);",

            h1 {
                class: "mt-6 text-2xl",
                "User Location Map"
            }

            div {
                id: "map-wrapper",
                style: "height: var(--map-height); aspect-ratio: 16 / 9; width: var(--content-width);",
                class: "rounded-lg bg-gray-800 shadow-lg overflow-hidden",

                div {
                    id: "map",
                    style: "width: 100%; height: 100%;"
                }
            }
        }
    }
}
