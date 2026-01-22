#[cfg(feature = "server")]
use crate::CURRENT_TEMPERATURE;
#[cfg(feature = "server")]
use crate::TEMPERATURE_BERN;
use dioxus::prelude::*;
use rand::seq::IndexedRandom;
use serde::{Deserialize, Serialize};
use std::io;

const VOGUGURU_CSS: Asset = asset!(
    "/assets/styling/voguguru.css",
    AssetOptions::css().with_static_head(true)
);
const PATTERN_HOCH: Asset = asset!("/assets/svg/aare-guru-pattern-hoch.svg",);
const PATTERN_QUER: Asset = asset!("/assets/svg/aare-guru-pattern-quer.svg",);
const FONT_WOFF2: Asset = asset!("/assets/webfonts/2D81A6_0_0.woff2",);
const FONT_WOFF: Asset = asset!("/assets/webfonts/2D81A6_0_0.woff",);
const FONT_TTF: Asset = asset!("/assets/webfonts/2D81A6_0_0.ttf",);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct GuruData {
    temperature_value: String,
    temperature_bern: String,
    phrase: String,
}

#[server]
async fn get_guru_data() -> Result<GuruData, ServerFnError> {
    let bern_temp: Option<f64> = {
        let lock = TEMPERATURE_BERN.read().unwrap();
        *lock
    };

    let current_temp: Option<f64> = {
        let lock = CURRENT_TEMPERATURE.read().unwrap();
        *lock
    };

    if let Ok(file) = std::fs::File::open("data/phrases.json") {
        let json: serde_json::Value =
            serde_json::from_reader(file).expect("file should be proper JSON");
        for (key, val) in json.as_object().iter().flat_map(|d| d.iter()) {
            let limits = key.split("..").collect::<Vec<&str>>();
            if let Some(t) = current_temp {
                if limits[0].parse::<f64>().unwrap() < t && t <= limits[1].parse::<f64>().unwrap() {
                    if let Some(phrase) = val.as_array().unwrap().choose(&mut rand::rng()) {
                        return Ok(GuruData {
                            temperature_value: current_temp
                                .map(|t| format!("{:.1}", t))
                                .unwrap_or_else(|| "—".to_string()),
                            temperature_bern: bern_temp
                                .map(|t| format!("{:.1}", t))
                                .unwrap_or_else(|| "—".to_string()),
                            phrase: phrase.to_string().replace("\"", ""),
                        });
                    }
                }
            }
        }
    }

    Ok(GuruData {
        temperature_value: current_temp
            .map(|t| format!("{:.1}", t))
            .unwrap_or_else(|| "—".to_string()),
        temperature_bern: bern_temp
            .map(|t| format!("{:.1}", t))
            .unwrap_or_else(|| "—".to_string()),
        phrase: "Perfekt für e Sprung!".to_string(),
    })
}

pub fn VoguGuru() -> Element {
    let mut guru_data = use_resource(move || get_guru_data());

    // Set up periodic refresh every 10 seconds - only start after initial load
    use_effect(move || {
        let handle = spawn(async move {
            // Wait for initial load
            #[cfg(target_arch = "wasm32")]
            gloo_timers::future::sleep(std::time::Duration::from_secs(10)).await;

            loop {
                guru_data.restart();
                #[cfg(target_arch = "wasm32")]
                gloo_timers::future::sleep(std::time::Duration::from_secs(10)).await;
            }
        });
    });

    rsx! {
        document::Style {
            {format!(r#"
                @font-face {{
                    font-family: 'DINNextLTPro-Condensed';
                    src: url("{FONT_WOFF2}") format("woff2"),
                         url("{FONT_WOFF}") format("woff"),
                         url("{FONT_TTF}") format("truetype");
                    font-weight: normal;
                    font-style: normal;
                }}
            "#)}
        }
        document::Link { rel: "stylesheet", href: VOGUGURU_CSS }

        div {
            class: "aarewasser",
            style: format!(
                "background-image: url({}); --pattern-quer: url({});",
                PATTERN_HOCH,
                PATTERN_QUER
            ),
            div { class: "container text-center mt-5",
                match &*guru_data.read_unchecked() {
                    Some(Ok(data)) => rsx! {
                        div { class: "temperature_big",
                            div { class: "temp-wrapper",
                                span { id: "temperatureValue", "{data.temperature_value}" }
                                span { class: "degree_big", "°" }
                            }
                        }
                        p { "Voguhuustemp. (°C)" }

                        h3 { id: "phrase", "{data.phrase}" }

                        div { class: "temperature",
                            div { class: "temp-wrapper",
                                span { id: "temperatureBern", "{data.temperature_bern}" }
                                span { class: "degree", "°" }
                            }
                        }
                        p { "Temperatur i ca. 2 stung" }
                    },
                    Some(Err(e)) => rsx! { p { "Error: {e}" } },
                    None => rsx! { p { "Loading..." } }
                }
            }
        }
    }
}
