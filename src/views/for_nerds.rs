use dioxus::prelude::*;

use crate::views::home::get_stream_config;
#[cfg(target_arch = "wasm32")]
use js_sys::eval;
#[cfg(feature = "server")]
use serde_json::Value;

const MAP_CSS: Asset = asset!("/assets/leaflet/leaflet.css",);
const MAP_JS: Asset = asset!("/assets/leaflet/leaflet.js",);
const MAP_INIT_JS: Asset = asset!("/assets/js/map_init.js",);

#[cfg(feature = "server")]
fn collect_max_grid_units(value: &Value, max_grid_units: &mut f64) {
    if let Some(obj) = value.as_object() {
        if let Some(grid_pos) = obj.get("gridPos").and_then(|v| v.as_object()) {
            let y = grid_pos.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let h = grid_pos.get("h").and_then(|v| v.as_f64()).unwrap_or(0.0);
            *max_grid_units = (*max_grid_units).max(y + h);
        }

        for child in obj.values() {
            collect_max_grid_units(child, max_grid_units);
        }
        return;
    }

    if let Some(arr) = value.as_array() {
        for child in arr {
            collect_max_grid_units(child, max_grid_units);
        }
    }
}

#[server]
async fn get_nerds_dashboard_height() -> Result<u32, ServerFnError> {
    const FALLBACK_DASHBOARD_HEIGHT: u32 = 3200;

    #[cfg(feature = "server")]
    {
        let cfg = get_stream_config().await?;
        let url = format!(
            "{}/api/public/dashboards/{}",
            cfg.grafana_base_url.trim_end_matches('/'),
            cfg.grafana_dashboard_nerds
        );

        let response = match reqwest::Client::new().get(url).send().await {
            Ok(response) => response,
            Err(e) => {
                eprintln!(
                    "Failed to request Grafana dashboard metadata: {}. Using fallback dashboard height {}px.",
                    e, FALLBACK_DASHBOARD_HEIGHT
                );
                return Ok(FALLBACK_DASHBOARD_HEIGHT);
            }
        };

        if !response.status().is_success() {
            eprintln!(
                "Grafana metadata request failed with HTTP {}. Using fallback dashboard height {}px.",
                response.status(),
                FALLBACK_DASHBOARD_HEIGHT
            );
            return Ok(FALLBACK_DASHBOARD_HEIGHT);
        }

        let json: Value = match response.json().await {
            Ok(json) => json,
            Err(e) => {
                eprintln!(
                    "Failed to parse Grafana metadata JSON: {}. Using fallback dashboard height {}px.",
                    e,
                    FALLBACK_DASHBOARD_HEIGHT
                );
                return Ok(FALLBACK_DASHBOARD_HEIGHT);
            }
        };

        let mut max_grid_units = 0.0_f64;
        collect_max_grid_units(&json, &mut max_grid_units);

        if max_grid_units <= 0.0 {
            eprintln!(
                "Grafana metadata had no grid layout. Using fallback dashboard height {}px.",
                FALLBACK_DASHBOARD_HEIGHT
            );
            return Ok(FALLBACK_DASHBOARD_HEIGHT);
        }

        // In practice, embedded public dashboards need a larger unit conversion than 30px.
        // Keep this configurable so deployments can tune once without code changes.
        let row_px = std::env::var("GRAFANA_NERDS_ROW_PX")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(38.0);
        let padding_px = std::env::var("GRAFANA_NERDS_PADDING_PX")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(120.0);

        let px = (max_grid_units * row_px + padding_px).round() as u32;
        let clamped = px.clamp(1800, 12000);
        println!(
            "Computed Grafana dashboard height from metadata: {}px (max_grid_units={}, row_px={}, padding_px={})",
            clamped, max_grid_units, row_px, padding_px
        );
        return Ok(clamped);
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Not running on server"))
    }
}

#[component]
pub fn ForNerds() -> Element {
    let config = use_resource(|| async move { get_stream_config().await.ok() });
    let dashboard_height =
        use_resource(|| async move { get_nerds_dashboard_height().await.unwrap_or(3200) });

    let config_value = config.read();
    let Some(Some(cfg)) = config_value.as_ref() else {
        return rsx! {
            div { "Loading…" }
        };
    };

    let grafana_url = format!(
        "{}/public-dashboards/{}?kiosk",
        cfg.grafana_base_url.trim_end_matches('/'),
        cfg.grafana_dashboard_nerds
    );
    let dashboard_height_px = dashboard_height.read().as_ref().copied().unwrap_or(3200);

    use_effect(|| {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = eval(
                r#"
            if (!window.__forNerdsMapInitScheduled) {
                window.__forNerdsMapInitScheduled = true;
                setTimeout(() => {
                    if (window.initLeafletMap) {
                        window.initLeafletMap();
                    }
                }, 300);
            }
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
            iframe {
                src: grafana_url,
                style: format!("width: var(--content-width); height: {}px; border: none;", dashboard_height_px),
                class: "rounded-lg shadow-lg bg-white",
                referrerpolicy: "no-referrer",
                scrolling: "no",
            }
        }
    }
}
