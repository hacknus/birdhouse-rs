use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

const VOGUGURU_CSS: Asset = asset!("/assets/styling/voguguru.css");
const PATTERN_HOCH: Asset = asset!("/assets/svg/aare-guru-pattern-hoch.svg");
const PATTERN_QUER: Asset = asset!("/assets/svg/aare-guru-pattern-quer.svg");
const FONT_WOFF2: Asset = asset!("/assets/webfonts/2D81A6_0_0.woff2");
const FONT_WOFF: Asset = asset!("/assets/webfonts/2D81A6_0_0.woff");
const FONT_TTF: Asset = asset!("/assets/webfonts/2D81A6_0_0.ttf");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct GuruData {
    temperature_value: String,
    temperature_bern: String,
    phrase: String,
}

#[server]
async fn get_guru_data() -> Result<GuruData, ServerFnError> {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    let temp_value = rng.gen_range(15.0..25.0);
    let temp_bern = rng.gen_range(15.0..25.0);

    Ok(GuruData {
        temperature_value: format!("{:.1}", temp_value),
        temperature_bern: format!("{:.1}", temp_bern),
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
        // document::Link { rel: "stylesheet", href: VOGUGURU_CSS }

        div {
            class: "aarewasser",
            style: "background-image: url('{PATTERN_HOCH}'); --pattern-quer: url('{PATTERN_QUER}');",
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
