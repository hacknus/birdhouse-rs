use crate::Route;
use dioxus::prelude::*;
use dioxus_router::{use_navigator, Link, Outlet};

const NAVBAR_CSS: Asset = asset!("/assets/styling/navbar.css");

#[component]
pub fn Navbar() -> Element {
    let navigator = use_navigator();
    let mut dropdown_value = use_signal(|| String::new());

    // Initialize the select to reflect the current path on first render (wasm only)
    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                if let Ok(path) = window.location().pathname() {
                    let v = if path.starts_with("/gallery") {
                        "gallery"
                    } else if path.starts_with("/making") {
                        "making"
                    } else if path.starts_with("/vogu") {
                        "vogu"
                    } else {
                        "home"
                    };
                    dropdown_value.set(v.to_string());
                }
            }
        }
    });

    rsx! {
        // document::Link { rel: "stylesheet", href: NAVBAR_CSS }

        div {
            id: "navbar",

            span { class: "nav-brand", "vögeli" }

            nav {
                class: "nav-links",
                Link { to: Route::Home {}, "Home" }
                Link { to: Route::Gallery {}, "Gallery" }
                Link { to: Route::MakingOf {}, "Making of" }
                Link { to: Route::VoguGuru {}, "vogu.guru" }
            }

            div {
                class: "nav-dropdown-shell",
                select {
                    class: "nav-dropdown",
                    value: dropdown_value(),
                    onchange: {
                        let navigator = navigator.clone();
                        let mut dropdown_value = dropdown_value.clone();
                        move |evt| {
                            let value = evt.value();
                            if value.is_empty() {
                                return;
                            }

                            let target = match value.as_str() {
                                "home" => Some(Route::Home {}),
                                "gallery" => Some(Route::Gallery {}),
                                "making" => Some(Route::MakingOf {}),
                                "vogu" => Some(Route::VoguGuru {}),
                                _ => None,
                            };

                            if let Some(route) = target {
                                let _ = navigator.push(route);
                            }

                            // keep the selected value so the select shows the current page
                            dropdown_value.set(value.clone());
                        }
                    },
                    option { value: "home", "Home" }
                    option { value: "gallery", "Gallery" }
                    option { value: "making", "Making of" }
                    option { value: "vogu", "vogu.guru" }
                }
            }
        }

        Outlet::<Route> {}
    }
}