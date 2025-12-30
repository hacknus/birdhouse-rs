use dioxus::prelude::*;
use views::{Blog, Gallery, MakingOf, VoguGuru, Home, Navbar};

mod components;
mod views;
mod api;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Navbar)]
        #[route("/")]
        Home {},

        #[route("/gallery")]
        Gallery {},

        #[route("/making-of")]
        MakingOf {},

        #[route("/vogu.guru")]
        VoguGuru {},

        #[route("/blog/:id")]
        Blog { id: i32 },
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/styling/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        Router::<Route> {}
    }
}

#[cfg(feature = "server")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use dioxus::prelude::*;
    use std::net::SocketAddr;
    use tower_http::services::ServeDir;
    use axum::{
        routing::get,
        response::Redirect,
    };

    dotenv::dotenv().ok();

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8080);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    println!("Listening on http://{}", addr);

    let router = Router::new()
        .route("/voegeli", get(|| async { Redirect::temporary("/") }))
        .serve_dioxus_application(ServeConfig::default(), App)
        .nest_service("/assets", ServeDir::new("public/assets"))
        .nest_service("/gallery_cache", ServeDir::new("public/gallery_cache"));

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, router.into_make_service())
        .await
        .unwrap();
}


#[cfg(not(feature = "server"))]
fn main() {
    dioxus::launch(App);
}
