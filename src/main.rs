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
const MAIN_CSS: Asset = asset!(
    "/assets/styling/main.css",
    AssetOptions::css().with_static_head(true)
);
const NAVBAR_CSS: Asset = asset!(
    "/assets/styling/navbar.css",
    AssetOptions::css().with_static_head(true)
);
const TAILWIND_CSS: Asset = asset!(
    "/assets/tailwind.css",
    AssetOptions::css().with_static_head(true)
);

#[component]
fn App() -> Element {
    let mut tcp_state = use_context_provider(|| tcp_state::TcpState::new());

    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        tcp_state.init_websocket();
    });

    // Ensure toast container exists in the DOM (wasm only)
    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if document.get_element_by_id("__dx-toast-decor").is_none() {
                    if let Ok(div) = document.create_element("div") {
                        let _ = div.set_attribute("id", "__dx-toast-decor");
                        if let Some(body) = document.body() {
                            let _ = body.append_child(&div);
                        }
                    }
                }
            }
        }
    });

    rsx! {
        document::Title { "vögeli" }
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: NAVBAR_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        Router::<Route> {}
    }
}

#[cfg(feature = "server")]
#[tokio::main]
async fn main() {
    use api::gallery::upload_image_multipart;
    use axum::routing::post;
    use axum::Router;
    use axum::{
        extract::ws::{WebSocket, WebSocketUpgrade},
        response::IntoResponse,
        response::Redirect,
        routing::get,
    };
    use dioxus::prelude::*;
    use futures::stream;
    use influxdb2::{models::DataPoint, Client as InfluxClient};
    use once_cell::sync::Lazy;
    use std::net::SocketAddr;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tokio::sync::broadcast;
    use tokio::time::{interval, Duration};
    use tower_http::services::ServeDir;

    static ACTIVE_USERS: Lazy<AtomicUsize> = Lazy::new(|| AtomicUsize::new(0));

    static TCP_BROADCAST: Lazy<broadcast::Sender<String>> = Lazy::new(|| {
        let (tx, _) = broadcast::channel(100);
        tx
    });

    async fn tcp_websocket_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
        ws.on_upgrade(handle_tcp_socket)
    }

    async fn handle_tcp_socket(mut socket: WebSocket) {
        ACTIVE_USERS.fetch_add(1, Ordering::SeqCst);
        println!(
            "User connected, active users = {}",
            ACTIVE_USERS.load(Ordering::Relaxed)
        );

        let mut rx = TCP_BROADCAST.subscribe();
        let mut ping = interval(Duration::from_secs(20));

        loop {
            tokio::select! {
                _ = ping.tick() => {
                    if socket
                        .send(axum::extract::ws::Message::Ping(vec![].into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }

                msg = rx.recv() => {
                    if let Ok(message) = msg {
                        if socket
                            .send(axum::extract::ws::Message::Text(message.into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                }

                result = socket.recv() => {
                    if result.is_none() {
                        break;
                    }
                }
            }
        }

        ACTIVE_USERS.fetch_sub(1, Ordering::SeqCst);
        println!(
            "User disconnected, active users = {}",
            ACTIVE_USERS.load(Ordering::Relaxed)
        );
    }

    dotenv::dotenv().ok();

    let influx_url = std::env::var("INFLUXDB_URL").expect("INFLUXDB_URL not set");
    let influx_org = std::env::var("INFLUXDB_ORG").expect("INFLUXDB_ORG not set");
    let influx_token = std::env::var("INFLUXDB_WRITE_TOKEN")
        .expect("INFLUXDB_WRITE_TOKEN not set")
        .trim()
        .to_string();

    dbg!(&influx_token);
    let influx_bucket = std::env::var("INFLUXDB_BUCKET").expect("INFLUXDB_BUCKET not set");

    let influx = InfluxClient::new(influx_url, influx_org, influx_token);

    // Initialize TCP connection with encryption key
    if let (Ok(tcp_addr), Ok(tcp_key)) = (
        std::env::var("TCP_SERVER_ADDR"),
        std::env::var("TCP_ENCRYPTION_KEY"),
    ) {
        match tcp_client::connect(&tcp_addr, &tcp_key.trim()).await {
            Ok(_) => println!("Connected to TCP server: {}", tcp_addr),
            Err(e) => eprintln!("Failed to connect to TCP server: {}", e),
        }
    }

    tokio::spawn(async {
        let mut rx = tcp_client::subscribe_to_tcp_messages();
        while let Ok(message) = rx.recv().await {
            let _ = TCP_BROADCAST.send(message);
        }
    });

    tokio::spawn({
        let influx = influx.clone();
        let bucket = influx_bucket.to_string();

        async move {
            let mut ticker = interval(Duration::from_secs(10));

            loop {
                ticker.tick().await;

                let users = ACTIVE_USERS.load(Ordering::Relaxed);

                let point = DataPoint::builder("voegeli")
                    .field("visitors", users as i64)
                    .build()
                    .unwrap();

                if let Err(e) = influx.write(&bucket, stream::iter(vec![point])).await {
                    eprintln!("InfluxDB write failed: {}", e);
                }
            }
        }
    });

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8080);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    println!("Listening on http://{}", addr);

    let router = Router::new()
        .route("/api/upload_image", post(upload_image_multipart))
        .route("/voegeli", get(|| async { Redirect::temporary("/") }))
        .route("/ws/tcp", get(tcp_websocket_handler))
        .nest_service("/assets", ServeDir::new("public/assets"))
        .nest_service("/gallery_cache", ServeDir::new("public/gallery_cache"))
        .serve_dioxus_application(ServeConfig::default(), App);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, router.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "server"))]
fn main() {
    dioxus::launch(App);
}
