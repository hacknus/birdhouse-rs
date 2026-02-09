use dioxus::prelude::*;

use crate::views::home::get_stream_config;
#[cfg(target_arch = "wasm32")]
use js_sys::eval;

const MAP_CSS: Asset = asset!("/assets/leaflet/leaflet.css",);
const MAP_JS: Asset = asset!("/assets/leaflet/leaflet.js",);
const MAP_INIT_JS: Asset = asset!("/assets/js/map_init.js",);
#[component]
pub fn ForNerds() -> Element {
    let config = use_resource(|| async move { get_stream_config().await.ok() });

    let config_value = config.read();
    let Some(Some(cfg)) = config_value.as_ref() else {
        return rsx! {
            div { "Loading…" }
        };
    };

    let grafana_url = format!(
        "{}/public-dashboards/{}",
        cfg.grafana_base_url.trim_end_matches('/'),
        cfg.grafana_dashboard_nerds
    );

    use_effect(|| {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = eval(
                r#"
            setTimeout(() => {
                if (window.initLeafletMap) {
                    window.initLeafletMap();
                }
            }, 300);
        "#,
            );
        }
    });

    rsx! {
        document::Link { rel: "stylesheet", href: MAP_CSS }
        document::Script { src: MAP_JS }
        document::Script { src: MAP_INIT_JS }

        div {
            class: "w-full flex flex-col items-center gap-10 px-4",
            style: "--content-width: min(100%, 1280px); --map-height: calc(var(--content-width) * 9 / 16);",

            h1 {
                class: "mt-6 text-2xl",
                "User Location Map"
            }

            // MAP
            div {
                id: "map-wrapper",
                style: "height: var(--map-height); aspect-ratio: 16 / 9; width: var(--content-width);",
                class: "rounded-lg bg-gray-800 shadow-lg overflow-hidden",

                div {
                    id: "map",
                    style: "width: 100%; height: 100%;"
                }
            }

            // DASHBOARD
            div {
                style: "width: var(--content-width);",
                class: "rounded-lg shadow-lg overflow-hidden bg-white aspect-video md:aspect-video h-[80vh] md:h-auto",
                iframe {
                    src: grafana_url,
                    style: "width: 100%; height: 100%; border: none;",
                    referrerpolicy: "no-referrer",
                }
            }
        }
    }
}
