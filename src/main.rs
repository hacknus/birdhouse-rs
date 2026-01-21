#[cfg(feature = "server")]
use axum::http::HeaderMap;
use dashmap::DashMap;
use dioxus::prelude::*;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use uuid::Uuid;
use views::{Blog, ForNerds, Gallery, Home, MakingOf, Navbar, VoguGuru};

#[cfg(feature = "server")]
use axum::extract::ConnectInfo;

mod api;
mod components;
mod views;

#[cfg(feature = "server")]
mod tcp_client;
mod tcp_state;

#[cfg(feature = "server")]
use std::{fs, path::Path};

#[cfg(feature = "server")]
const LOCATION_FILE: &str = "data/locations.json";

#[cfg(feature = "server")]
static USER_LOCATIONS: Lazy<DashMap<Uuid, UserLocation>> = Lazy::new(DashMap::new);

#[cfg(feature = "server")]
static STORED_LOCATIONS: Lazy<DashMap<String, StoredLocation>> = Lazy::new(DashMap::new);

#[derive(Serialize, Deserialize, Clone)]
struct StoredLocation {
    lat: f64,
    lng: f64,
    country: String,
    city: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct UserLocation {
    id: String,
    key: String, // "<city>,<country>"
    lat: f64,
    lng: f64,
    country: String,
    city: String,
    connected_at: i64,
}

#[derive(Deserialize, Clone)]
struct IpGeoResponse {
    lat: f64,
    lon: f64,
    country: String,
    city: String,
}

// Messages sent over WS to the browser map
#[derive(Serialize)]
#[serde(tag = "type")]
enum WsMsg {
    #[serde(rename = "past")]
    Past {
        key: String,
        lat: f64,
        lng: f64,
        country: String,
        city: String,
        past: bool, // always true; convenient for your JS
    },

    #[serde(rename = "connect")]
    Connect {
        id: String,
        key: String,
        lat: f64,
        lng: f64,
        country: String,
        city: String,
        connected_at: i64,
    },

    #[serde(rename = "disconnect")]
    Disconnect { id: String, key: String },
}

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

        #[route("/for_nerds")]
        ForNerds {},

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

#[cfg(feature = "server")]
fn load_locations_from_disk() -> Vec<StoredLocation> {
    match fs::read_to_string(LOCATION_FILE) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

#[cfg(feature = "server")]
fn save_locations_to_disk(locations: &[StoredLocation]) {
    use std::path::Path;

    let path = Path::new(LOCATION_FILE);

    if let Some(parent) = path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            eprintln!("Failed to create directory {:?}: {}", parent, e);
            return;
        }
    }

    match fs::write(path, serde_json::to_string_pretty(locations).unwrap()) {
        Ok(_) => println!("Saved {} locations to {:?}", locations.len(), path),
        Err(e) => eprintln!("Failed to write locations file: {}", e),
    }
}

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
async fn geo_lookup(ip: &IpAddr) -> Option<IpGeoResponse> {
    if is_private_ip(ip) || ip.is_loopback() {
        println!("Skipping geo lookup for private IP: {}", ip);
        return None;
    }

    let url = format!("http://ip-api.com/json/{}", ip);
    let resp = reqwest::get(&url).await.ok()?;
    let geo = resp.json::<IpGeoResponse>().await.ok()?;
    Some(geo)
}


fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_unspecified()
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unique_local()
                || v6.is_unspecified()
        }
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
    use futures::stream;
    use influxdb2::{models::DataPoint, Client as InfluxClient};
    use std::net::SocketAddr;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::broadcast;
    use tokio::time::{interval, Duration};
    use tower_http::services::ServeDir;

    static ACTIVE_USERS: Lazy<AtomicUsize> = Lazy::new(|| AtomicUsize::new(0));

    static TCP_BROADCAST: Lazy<broadcast::Sender<String>> = Lazy::new(|| {
        let (tx, _) = broadcast::channel(200);
        tx
    });

    // ---- Boot: load past locations into memory
    {
        let past_locations = load_locations_from_disk();
        for loc in past_locations {
            let key = format!("{},{}", loc.city, loc.country);
            STORED_LOCATIONS.insert(key, loc);
        }

        let all: Vec<StoredLocation> = STORED_LOCATIONS.iter().map(|e| e.value().clone()).collect();

        save_locations_to_disk(&all);

        println!("Loaded {} past locations from disk", STORED_LOCATIONS.len());
    }

    fn extract_real_ip(headers: &HeaderMap) -> Option<IpAddr> {
        let candidates = [
            "cf-connecting-ip",
            "x-real-ip",
            "x-forwarded-for",
            "forwarded",
        ];

        for key in candidates {
            if let Some(value) = headers.get(key) {
                if let Ok(s) = value.to_str() {
                    // X-Forwarded-For can be: client, proxy1, proxy2
                    let first = s.split(',').next().unwrap().trim();
                    if let Ok(ip) = first.parse::<IpAddr>() {
                        if !is_private_ip(&ip) {
                            return Some(ip);
                        }
                    }
                }
            }
        }

        None
    }

    async fn tcp_websocket_handler(
        ws: WebSocketUpgrade,
        headers: axum::http::HeaderMap,
        ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ) -> impl IntoResponse {
        let ip = extract_real_ip(&headers).unwrap_or_else(|| addr.ip());
        ws.on_upgrade(move |socket| handle_tcp_socket(socket, ip))
    }

    async fn handle_tcp_socket(mut socket: WebSocket, ip: IpAddr) {
        println!("New WS from IP: {}", ip);

        let user_id = Uuid::new_v4();
        let mut this_user: Option<UserLocation> = None;

        // Resolve geo + build current user entry
        if let Some(geo) = geo_lookup(&ip).await {
            let key = format!("{},{}", geo.city, geo.country);

            // Persist unique location if new
            if !STORED_LOCATIONS.contains_key(&key) {
                let stored = StoredLocation {
                    lat: geo.lat,
                    lng: geo.lon,
                    country: geo.country.clone(),
                    city: geo.city.clone(),
                };
                STORED_LOCATIONS.insert(key.clone(), stored);

                let all: Vec<StoredLocation> =
                    STORED_LOCATIONS.iter().map(|e| e.value().clone()).collect();
                save_locations_to_disk(&all);

                println!("Persisted new location: {}", key);
            }

            let loc = UserLocation {
                id: user_id.to_string(),
                key: key.clone(),
                lat: geo.lat,
                lng: geo.lon,
                country: geo.country,
                city: geo.city,
                connected_at: chrono::Utc::now().timestamp(),
            };

            USER_LOCATIONS.insert(user_id, loc.clone());
            this_user = Some(loc.clone());

            // Broadcast "connect" so *all* clients increment current counts
            let msg = WsMsg::Connect {
                id: loc.id.clone(),
                key: loc.key.clone(),
                lat: loc.lat,
                lng: loc.lng,
                country: loc.country.clone(),
                city: loc.city.clone(),
                connected_at: loc.connected_at,
            };
            let _ = TCP_BROADCAST.send(serde_json::to_string(&msg).unwrap());
        } else {
            // Still count as active, but won't appear on map without geo
            println!("Geo lookup failed for IP: {}", ip);
        }

        ACTIVE_USERS.fetch_add(1, Ordering::SeqCst);
        println!(
            "User connected, active users = {}",
            ACTIVE_USERS.load(Ordering::Relaxed)
        );

        // Subscribe after broadcasting connect is fine; new client will also receive snapshot below.
        let mut rx = TCP_BROADCAST.subscribe();
        let mut ping = interval(Duration::from_secs(20));

        // ---- On new socket: send all past locations first (gray)
        for entry in STORED_LOCATIONS.iter() {
            let loc = entry.value();
            let key = format!("{},{}", loc.city, loc.country);
            let msg = WsMsg::Past {
                key,
                lat: loc.lat,
                lng: loc.lng,
                country: loc.country.clone(),
                city: loc.city.clone(),
                past: true,
            };

            if socket
                .send(axum::extract::ws::Message::Text(
                    serde_json::to_string(&msg).unwrap().into(),
                ))
                .await
                .is_err()
            {
                // Client gone
                break;
            }
        }

        // ---- Then send snapshot of currently connected users (blue)
        for entry in USER_LOCATIONS.iter() {
            let u = entry.value();
            let msg = WsMsg::Connect {
                id: u.id.clone(),
                key: u.key.clone(),
                lat: u.lat,
                lng: u.lng,
                country: u.country.clone(),
                city: u.city.clone(),
                connected_at: u.connected_at,
            };

            if socket
                .send(axum::extract::ws::Message::Text(
                    serde_json::to_string(&msg).unwrap().into(),
                ))
                .await
                .is_err()
            {
                break;
            }
        }

        // Main loop: ping + broadcast fanout
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

        // ---- Disconnect: remove from active map and notify everyone
        if let Some(u) = this_user {
            USER_LOCATIONS.remove(&user_id);

            let msg = WsMsg::Disconnect {
                id: u.id.clone(),
                key: u.key.clone(),
            };
            let _ = TCP_BROADCAST.send(serde_json::to_string(&msg).unwrap());
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
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

#[cfg(not(feature = "server"))]
fn main() {
    dioxus::launch(App);
}
