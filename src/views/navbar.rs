use crate::Route;
use dioxus::prelude::*;
use dioxus_router::{use_navigator, use_route, Link, Outlet};

const NAVBAR_CSS: Asset = asset!("/assets/styling/navbar.css");

#[component]
pub fn Navbar() -> Element {
    let navigator = use_navigator();
    let current_route = use_route::<Route>();

    let show_nerds = matches!(current_route, Route::ForNerds {});

    let selected = match current_route {
        Route::Home {} => "home",
        Route::Gallery {} => "gallery",
        Route::MakingOf {} => "making",
        Route::VoguGuru {} => "vogu",
        Route::ForNerds {} => "nerds",
    };

    rsx! {
        document::Link { rel: "stylesheet", href: NAVBAR_CSS }

        div {
            id: "navbar",

            span { class: "nav-brand", "vögeli" }

            nav {
                class: "nav-links",
                Link { to: Route::Home {}, "Home" }
                Link { to: Route::Gallery {}, "Gallery" }
                Link { to: Route::MakingOf {}, "Making of" }
                Link { to: Route::VoguGuru {}, "vogu.guru" }

                if show_nerds {
                    Link { to: Route::ForNerds {}, "For Nerds" }
                }
            }

            div {
                class: "nav-dropdown-shell",
                select {
                    class: "nav-dropdown",
                    value: selected,

                    onchange: move |evt| {
                        let value = evt.value();
                        let target = match value.as_str() {
                            "home" => Some(Route::Home {}),
                            "gallery" => Some(Route::Gallery {}),
                            "making" => Some(Route::MakingOf {}),
                            "vogu" => Some(Route::VoguGuru {}),
                            "nerds" => Some(Route::ForNerds {}),
                            _ => None,
                        };

                        if let Some(route) = target {
                            let _ = navigator.push(route);
                        }
                    },

                    option { value: "home", "Home" }
                    option { value: "gallery", "Gallery" }
                    option { value: "making", "Making of" }
                    option { value: "vogu", "vogu.guru" }

                    if show_nerds {
                        option { value: "nerds", "For Nerds" }
                    }
                }
            }
        }

        Outlet::<Route> {}
    }
}