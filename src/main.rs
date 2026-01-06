use dioxus::prelude::*;
use views::{Blog, Gallery, Home, MakingOf, Navbar, VoguGuru};

mod api;
mod components;
mod views;

#[cfg(feature = "server")]
mod tcp_client;
mod tcp_state;

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
    let tcp_state = use_context_provider(|| tcp_state::TcpState::new());

    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        tcp_state.init_websocket();
    });

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
    use axum::{
        extract::ws::{WebSocket, WebSocketUpgrade},
        response::IntoResponse,
        response::Redirect,
        routing::get,
    };
    use dioxus::prelude::*;
    use once_cell::sync::Lazy;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use tokio::sync::broadcast;
    use tower_http::services::ServeDir;

    static TCP_BROADCAST: Lazy<broadcast::Sender<String>> = Lazy::new(|| {
        let (tx, _) = broadcast::channel(100);
        tx
    });

    async fn tcp_websocket_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
        ws.on_upgrade(handle_tcp_socket)
    }

    async fn handle_tcp_socket(mut socket: WebSocket) {
        let mut rx = TCP_BROADCAST.subscribe();

        while let Ok(message) = rx.recv().await {
            if socket
                .send(axum::extract::ws::Message::Text(message.into()))
                .await
                .is_err()
            {
                break;
            }
        }
    }

    dotenv::dotenv().ok();

    // Initialize TCP connection with encryption key
    if let (Ok(tcp_addr), Ok(tcp_key)) = (
        std::env::var("TCP_SERVER_ADDR"),
        std::env::var("TCP_ENCRYPTION_KEY"),
    ) {
        match tcp_client::connect(&tcp_addr, &tcp_key.trim()) {
            Ok(_) => println!("Connected to TCP server: {}", tcp_addr),
            Err(e) => eprintln!("Failed to connect to TCP server: {}", e),
        }
    } else {
        eprintln!("TCP_SERVER_ADDR or TCP_ENCRYPTION_KEY not set in environment");
    }

    tokio::spawn(async {
        let mut rx = tcp_client::subscribe_to_tcp_messages();
        while let Ok(message) = rx.recv().await {
            let _ = TCP_BROADCAST.send(message);
        }
    });

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8080);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    println!("Listening on http://{}", addr);

    let router = Router::new()
        .route("/voegeli", get(|| async { Redirect::temporary("/") }))
        .route("/ws/tcp", get(tcp_websocket_handler)) // New endpoint
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
