use crate::Route;
use dioxus::prelude::*;
use dioxus_router::{use_navigator, use_route, Link, Outlet};

const NAVBAR_CSS: Asset = asset!("/assets/styling/navbar.css");

#[component]
fn NavItem(to: Route, label: &'static str, current_route: Route) -> Element {
    rsx! {
        Link {
            to: to.clone(),
            onclick: move |_| {
                if current_route == to {
                    #[cfg(target_arch = "wasm32")]
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().reload();
                    }
                }
            },
            "{label}"
        }
    }
}

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
                NavItem { to: Route::Home {}, label: "Home", current_route: current_route.clone() }
                NavItem { to: Route::Gallery {}, label: "Gallery", current_route: current_route.clone() }
                NavItem { to: Route::Birds {}, label: "Birds", current_route: current_route.clone() }
                // Link { to: Route::MakingOf {}, "Making of" }
                NavItem { to: Route::HowItWorks {}, label: "How It Works", current_route: current_route.clone() }
                NavItem { to: Route::Newsletter {}, label: "Newsletter", current_route: current_route.clone() }
                NavItem { to: Route::VoguGuru {}, label: "vogu.guru", current_route: current_route.clone() }

                if show_nerds {
                    NavItem { to: Route::ForNerds {}, label: "For Nerds", current_route: current_route.clone() }
                }
                if show_admin {
                    NavItem { to: Route::Admin {}, label: "Admin", current_route: current_route.clone() }
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
