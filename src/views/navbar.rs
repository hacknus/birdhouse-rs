use crate::Route;
use dioxus::prelude::*;
use dioxus_router::{use_navigator, use_route, Link, Outlet};

const NAVBAR_CSS: Asset = asset!("/assets/styling/navbar.css");

#[component]
pub fn Navbar() -> Element {
    let navigator = use_navigator();
    let current_route = use_route::<Route>();

    let show_nerds = matches!(current_route, Route::ForNerds {});
    let show_admin = matches!(current_route, Route::Admin {});

    let selected = match current_route {
        Route::Home {} => "home",
        Route::Gallery {} => "gallery",
        Route::MakingOf {} => "making",
        Route::HowItWorks {} => "how",
        Route::Birds {} => "birds",
        Route::VoguGuru {} => "vogu",
        Route::Newsletter {} => "newsletter",
        Route::Unsubscribe { .. } => "newsletter",
        Route::ForNerds {} => "nerds",
        Route::Admin {} => "admin",
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
                Link { to: Route::Birds {}, "Birds" }
                // Link { to: Route::MakingOf {}, "Making of" }
                Link { to: Route::HowItWorks {}, "How It Works" }
                Link { to: Route::Newsletter {}, "Newsletter" }
                Link { to: Route::VoguGuru {}, "vogu.guru" }

                if show_nerds {
                    Link { to: Route::ForNerds {}, "For Nerds" }
                }
                if show_admin {
                    Link { to: Route::Admin {}, "Admin" }
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
                            "birds" => Some(Route::Birds {}),
                            // "making" => Some(Route::MakingOf {}),
                            "how" => Some(Route::HowItWorks {}),
                            "newsletter" => Some(Route::Newsletter {}),
                            "vogu" => Some(Route::VoguGuru {}),
                            "nerds" => Some(Route::ForNerds {}),
                            "admin" => Some(Route::Admin {}),
                            _ => None,
                        };

                        if let Some(route) = target {
                            let _ = navigator.push(route);
                        }
                    },

                    option { value: "home", "Home" }
                    option { value: "gallery", "Gallery" }
                    option { value: "birds", "Birds" }
                    // option { value: "making", "Making of" }
                    option { value: "how", "How It Works" }
                    option { value: "newsletter", "Newsletter" }
                    option { value: "vogu", "vogu.guru" }

                    if show_nerds {
                        option { value: "nerds", "For Nerds" }
                    }
                    if show_admin {
                        option { value: "admin", "Admin" }
                    }
                }
            }
        }

        Outlet::<Route> {}
    }
}
