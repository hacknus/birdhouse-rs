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
fn collect_grid_units(value: &Value, max_grid_units: &mut f64, sum_grid_units: &mut f64) {
    if let Some(obj) = value.as_object() {
        if let Some(grid_pos) = obj.get("gridPos").and_then(|v| v.as_object()) {
            let y = grid_pos.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let h = grid_pos.get("h").and_then(|v| v.as_f64()).unwrap_or(0.0);
            *max_grid_units = (*max_grid_units).max(y + h);
            *sum_grid_units += h.max(0.0);
        }

        for child in obj.values() {
            collect_grid_units(child, max_grid_units, sum_grid_units);
        }
        return;
    }

    if let Some(arr) = value.as_array() {
        for child in arr {
            collect_grid_units(child, max_grid_units, sum_grid_units);
        }
    }
}

#[server]
async fn get_nerds_dashboard_heights() -> Result<(u32, u32), ServerFnError> {
    const FALLBACK_DESKTOP_HEIGHT: u32 = 3200;
    const FALLBACK_MOBILE_HEIGHT: u32 = 5600;

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
                    "Failed to request Grafana dashboard metadata: {}. Using fallback heights desktop={}px mobile={}px.",
                    e, FALLBACK_DESKTOP_HEIGHT, FALLBACK_MOBILE_HEIGHT
                );
                return Ok((FALLBACK_DESKTOP_HEIGHT, FALLBACK_MOBILE_HEIGHT));
            }
        };

        if !response.status().is_success() {
            eprintln!(
                "Grafana metadata request failed with HTTP {}. Using fallback heights desktop={}px mobile={}px.",
                response.status(),
                FALLBACK_DESKTOP_HEIGHT,
                FALLBACK_MOBILE_HEIGHT
            );
            return Ok((FALLBACK_DESKTOP_HEIGHT, FALLBACK_MOBILE_HEIGHT));
        }

        let json: Value = match response.json().await {
            Ok(json) => json,
            Err(e) => {
                eprintln!(
                    "Failed to parse Grafana metadata JSON: {}. Using fallback heights desktop={}px mobile={}px.",
                    e,
                    FALLBACK_DESKTOP_HEIGHT,
                    FALLBACK_MOBILE_HEIGHT
                );
                return Ok((FALLBACK_DESKTOP_HEIGHT, FALLBACK_MOBILE_HEIGHT));
            }
        };

        let mut max_grid_units = 0.0_f64;
        let mut sum_grid_units = 0.0_f64;
        collect_grid_units(&json, &mut max_grid_units, &mut sum_grid_units);

        if max_grid_units <= 0.0 || sum_grid_units <= 0.0 {
            eprintln!(
                "Grafana metadata had no usable grid layout. Using fallback heights desktop={}px mobile={}px.",
                FALLBACK_DESKTOP_HEIGHT,
                FALLBACK_MOBILE_HEIGHT
            );
            return Ok((FALLBACK_DESKTOP_HEIGHT, FALLBACK_MOBILE_HEIGHT));
        }

        // Desktop: normal Grafana grid estimate from max y+h.
        let row_px = std::env::var("GRAFANA_NERDS_ROW_PX")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(38.0);
        let padding_px = std::env::var("GRAFANA_NERDS_PADDING_PX")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(120.0);

        // Mobile: approximate stacked layout from sum of all panel heights.
        let mobile_row_px = std::env::var("GRAFANA_NERDS_MOBILE_ROW_PX")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(36.0);
        let mobile_padding_px = std::env::var("GRAFANA_NERDS_MOBILE_PADDING_PX")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(260.0);

        let desktop_px = (max_grid_units * row_px + padding_px).round() as u32;
        let mobile_px = (sum_grid_units * mobile_row_px + mobile_padding_px).round() as u32;

        let desktop_clamped = desktop_px.clamp(1800, 12000);
        let mobile_clamped = mobile_px.clamp(2600, 24000);
        println!(
            "Computed Grafana dashboard heights from metadata: desktop={}px mobile={}px (max_grid_units={}, sum_grid_units={}, row_px={}, padding_px={}, mobile_row_px={}, mobile_padding_px={})",
            desktop_clamped,
            mobile_clamped,
            max_grid_units,
            sum_grid_units,
            row_px,
            padding_px,
            mobile_row_px,
            mobile_padding_px
        );
        return Ok((desktop_clamped, mobile_clamped));
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Not running on server"))
    }
}

#[component]
pub fn ForNerds() -> Element {
    let config = use_resource(|| async move { get_stream_config().await.ok() });
    let dashboard_heights =
        use_resource(|| async move { get_nerds_dashboard_heights().await.unwrap_or((3200, 5600)) });

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
    let (dashboard_height_px, computed_mobile_height_px) = dashboard_heights
        .read()
        .as_ref()
        .copied()
        .unwrap_or((3200, 5600));
    let mobile_min_height_px = std::env::var("GRAFANA_NERDS_MOBILE_MIN_HEIGHT")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(6200);
    let dashboard_height_mobile_px = computed_mobile_height_px.max(mobile_min_height_px);

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
        document::Style {
            {format!(r#"
                #for-nerds-grafana {{
                    height: var(--grafana-height-desktop);
                }}

                @media (max-width: 900px) {{
                    #for-nerds-grafana {{
                        height: var(--grafana-height-mobile);
                    }}
                }}
            "#)}
        }

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
                id: "for-nerds-grafana",
                src: grafana_url,
                style: format!("--grafana-height-desktop: {}px; --grafana-height-mobile: {}px; width: var(--content-width); border: none;", dashboard_height_px, dashboard_height_mobile_px),
                class: "rounded-lg shadow-lg bg-white",
                referrerpolicy: "no-referrer",
                scrolling: "no",
            }
        }
    }
}
