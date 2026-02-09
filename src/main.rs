#[cfg(feature = "server")]
use axum::extract::ConnectInfo;
#[cfg(feature = "server")]
use axum::http::HeaderMap;
#[cfg(feature = "server")]
use dashmap::DashMap;
use dioxus::prelude::*;
#[cfg(feature = "server")]
use influxdb2::models::Query as InfluxQuery;
#[cfg(feature = "server")]
use influxdb2::FromDataPoint;
#[cfg(feature = "server")]
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
#[cfg(feature = "server")]
use uuid::Uuid;
use views::{ForNerds, Gallery, Home, MakingOf, Navbar, VoguGuru};

mod api;
mod components;
mod views;

#[cfg(feature = "server")]
mod tcp_client;
mod tcp_state;

#[cfg(feature = "server")]
use axum::extract::Query;
#[cfg(feature = "server")]
use std::collections::HashMap;
#[cfg(feature = "server")]
use std::fs;
#[cfg(feature = "server")]
use std::sync::RwLock;

#[cfg(feature = "server")]
const LOCATION_FILE: &str = "data/locations.json";

#[cfg(feature = "server")]
pub static TEMPERATURE_BERN: Lazy<RwLock<Option<f64>>> = Lazy::new(|| RwLock::new(None));

#[cfg(feature = "server")]
pub static CURRENT_INSIDE_TEMPERATURE: Lazy<RwLock<Option<f64>>> = Lazy::new(|| RwLock::new(None));

#[cfg(feature = "server")]
pub static CURRENT_OUTSIDE_TEMPERATURE: Lazy<RwLock<Option<f64>>> = Lazy::new(|| RwLock::new(None));

#[cfg(feature = "server")]
static USER_LOCATIONS: Lazy<DashMap<Uuid, UserLocation>> = Lazy::new(DashMap::new);

#[cfg(feature = "server")]
static ACTIVE_SESSIONS: Lazy<DashMap<String, SessionEntry>> = Lazy::new(DashMap::new);

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
    key: String, // "<lat>,<long>"
    lat: f64,
    lng: f64,
    country: String,
    city: String,
    connected_at: i64,
}

#[cfg(feature = "server")]
#[derive(Clone)]
struct SessionEntry {
    connections: usize,
    last_seen: i64,
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
        if tcp_state.ws.read().is_none() {
            tcp_state.init_websocket();
        }
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
            v4.is_private() || v4.is_loopback() || v4.is_link_local() || v4.is_unspecified()
        }
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local() || v6.is_unspecified(),
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
    use once_cell::sync::Lazy;
    use std::net::SocketAddr;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::broadcast;
    use tokio::time::{interval, Duration};
    use tower_http::services::ServeDir;

    dotenv::dotenv().ok();

    static ACTIVE_USERS: Lazy<AtomicUsize> = Lazy::new(|| AtomicUsize::new(0));

    static TCP_BROADCAST: Lazy<broadcast::Sender<String>> = Lazy::new(|| {
        let (tx, _) = broadcast::channel(200);
        tx
    });

    #[cfg(feature = "server")]
    pub async fn get_access_token() -> Option<String> {
        use base64::{engine::general_purpose, Engine as _};

        let client_id = std::env::var("CLIENT_ID").ok()?;
        let client_secret = std::env::var("CLIENT_SECRET").ok()?;

        let credentials = format!("{}:{}", client_id, client_secret);
        let encoded = general_purpose::STANDARD.encode(credentials);

        let url = "https://api.srgssr.ch/oauth/v1/accesstoken?grant_type=client_credentials";

        let client = reqwest::Client::new();
        let res = match client
            .post(url)
            .header("Authorization", format!("Basic {}", encoded))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body("grant_type=client_credentials")
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Token request failed: {}", e);
                return None;
            }
        };

        let status = res.status();
        let text = res.text().await.unwrap_or_default();

        if !status.is_success() {
            eprintln!("Token HTTP {}: {}", status, text);
            return None;
        }

        let json: serde_json::Value = match serde_json::from_str(&text) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("Token JSON parse failed: {}", e);
                return None;
            }
        };

        let token = json.get("access_token")?.as_str()?.to_string();
        println!("Got access token");
        Some(token)
    }

    #[cfg(feature = "server")]
    async fn fetch_current_temperature(
        client: &influxdb2::Client,
        bucket: &str,
        field: &str,
    ) -> Option<f64> {
        let flux = format!(
            r#"
                from(bucket: "{bucket}")
                  |> range(start: -24h)
                  |> filter(fn: (r) => r._measurement == "voegeli")
                  |> filter(fn: (r) => r._field == "{field}")
                  |> last()
                "#,
            bucket = bucket
        );

        client.query_suggestions().await.ok();
        client.query_suggestions_name("some-name").await.ok();

        #[derive(FromDataPoint)]
        struct Measurement {
            value: f64,
        }
        impl Default for Measurement {
            fn default() -> Self {
                Self { value: 0f64 }
            }
        }

        let rows = client
            .query::<Measurement>(Some(InfluxQuery::new(flux)))
            .await
            .expect("Query failed");

        let latest_value = rows.into_iter().next().map(|m| m.value);
        latest_value
    }

    #[cfg(feature = "server")]
    pub async fn get_weather_forecast(token: &str, geolocation_id: &str) -> Option<f64> {
        let url = format!(
            "https://api.srgssr.ch/srf-meteo/v2/forecastpoint/{}",
            geolocation_id
        );

        let client = reqwest::Client::new();
        let res = match client
            .get(url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Weather request failed: {}", e);
                return None;
            }
        };

        let status = res.status();
        let text = res.text().await.unwrap_or_default();

        if !status.is_success() {
            eprintln!("Weather HTTP {}: {}", status, text);
            return None;
        }

        let json: serde_json::Value = match serde_json::from_str(&text) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("Weather JSON parse failed: {}", e);
                return None;
            }
        };

        let hours = json.get("hours")?.as_array()?;

        use chrono::{Timelike, Utc};

        let now = Utc::now();
        let mut best: Option<(chrono::DateTime<Utc>, f64)> = None;

        for entry in hours {
            let dt = entry.get("date_time")?.as_str()?;
            let parsed = chrono::DateTime::parse_from_rfc3339(dt)
                .ok()?
                .with_timezone(&Utc);

            if parsed > now {
                if let Some(temp) = entry.get("TTT_C")?.as_f64() {
                    match best {
                        Some((best_time, _)) if parsed >= best_time => {}
                        _ => best = Some((parsed, temp)),
                    }
                }
            }
        }

        if let Some((t, temp)) = best {
            println!("Closest forecast @ {} = {}", t, temp);
            return Some(temp);
        }

        eprintln!("No future forecast entries found");
        None
    }

    #[cfg(feature = "server")]
    {
        tokio::spawn(async {
            let geolocation_id = "46.9548,7.4320"; // Bern

            loop {
                match get_access_token().await {
                    Some(token) => match get_weather_forecast(&token, geolocation_id).await {
                        Some(temp) => {
                            let mut lock = TEMPERATURE_BERN.write().unwrap();
                            *lock = Some(temp);
                            println!("Updated Bern temperature: {}", temp);
                        }
                        None => {
                            eprintln!("Failed to get weather forecast");
                        }
                    },
                    None => {
                        eprintln!("Failed to get access token");
                    }
                }

                // Update every 1 hour
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        });
    }

    // ---- Boot: load past locations into memory
    {
        let past_locations = load_locations_from_disk();
        for loc in past_locations {
            let key = format!("{},{}", loc.lat, loc.lng);
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
        Query(params): Query<HashMap<String, String>>,
        headers: HeaderMap,
        ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ) -> impl IntoResponse {
        let role = params.get("role").cloned().unwrap_or("viewer".into());
        let session_id = params
            .get("session_id")
            .cloned()
            .unwrap_or_else(|| "missing".to_string());
        let ip = extract_real_ip(&headers).unwrap_or_else(|| addr.ip());
        ws.on_upgrade(move |socket| handle_tcp_socket(socket, ip, role, session_id))
    }

    async fn handle_tcp_socket(
        mut socket: WebSocket,
        ip: IpAddr,
        role: String,
        session_id: String,
    ) {
        println!("New WS from IP: {} (session_id={})", ip, session_id);

        let user_id = Uuid::new_v4();
        let mut this_user: Option<UserLocation> = None;
        let mut counted_viewer = false;

        // Resolve geo + build current user entry
        if let Some(geo) = geo_lookup(&ip).await {
            let key = format!("{},{}", geo.lat, geo.lon);

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

                println!("Persisted new location: {}/{}", geo.city, geo.country);
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

        if role == "viewer" {
            let now = chrono::Utc::now().timestamp();
            let mut entry = ACTIVE_SESSIONS
                .entry(session_id.clone())
                .or_insert(SessionEntry {
                    connections: 0,
                    last_seen: now,
                });
            entry.connections += 1;
            entry.last_seen = now;
            if entry.connections == 1 {
                ACTIVE_USERS.fetch_add(1, Ordering::SeqCst);
            }
            counted_viewer = true;
            println!(
                "User connected, active users = {} (session_id={}, connections={})",
                ACTIVE_USERS.load(Ordering::Relaxed),
                session_id,
                entry.connections
            );
        }

        // Subscribe after broadcasting connect is fine; new client will also receive snapshot below.
        let mut rx = TCP_BROADCAST.subscribe();
        let mut ping = interval(Duration::from_secs(20));

        // ---- On new socket: send all past locations first (gray)
        for entry in STORED_LOCATIONS.iter() {
            let loc = entry.value();
            let key = format!("{},{}", loc.lat, loc.lng);
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
                        println!("WS ping failed (session_id={})", session_id);
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
                            println!("WS send failed (session_id={})", session_id);
                            break;
                        }
                    }
                }

                result = socket.recv() => {
                    match result {
                        None => {
                            println!("WS recv None (session_id={})", session_id);
                            break;
                        }
                        Some(Ok(axum::extract::ws::Message::Close(frame))) => {
                            println!("WS recv Close {:?} (session_id={})", frame, session_id);
                            break;
                        }
                        Some(Ok(axum::extract::ws::Message::Text(text))) => {
                            if text == "hb" {
                                println!("WS recv heartbeat (session_id={})", session_id);
                            } else {
                                println!(
                                    "WS recv text len={} (session_id={})",
                                    text.len(),
                                    session_id
                                );
                            }
                            if counted_viewer {
                                if let Some(mut entry) = ACTIVE_SESSIONS.get_mut(&session_id) {
                                    entry.last_seen = chrono::Utc::now().timestamp();
                                }
                            }
                        }
                        Some(Ok(axum::extract::ws::Message::Binary(data))) => {
                            println!(
                                "WS recv binary len={} (session_id={})",
                                data.len(),
                                session_id
                            );
                            if counted_viewer {
                                if let Some(mut entry) = ACTIVE_SESSIONS.get_mut(&session_id) {
                                    entry.last_seen = chrono::Utc::now().timestamp();
                                }
                            }
                        }
                        Some(Ok(axum::extract::ws::Message::Ping(_))) => {
                            println!("WS recv Ping (session_id={})", session_id);
                            if counted_viewer {
                                if let Some(mut entry) = ACTIVE_SESSIONS.get_mut(&session_id) {
                                    entry.last_seen = chrono::Utc::now().timestamp();
                                }
                            }
                        }
                        Some(Ok(axum::extract::ws::Message::Pong(_))) => {
                            println!("WS recv Pong (session_id={})", session_id);
                            if counted_viewer {
                                if let Some(mut entry) = ACTIVE_SESSIONS.get_mut(&session_id) {
                                    entry.last_seen = chrono::Utc::now().timestamp();
                                }
                            }
                        }
                        Some(Err(err)) => {
                            println!("WS recv error {:?} (session_id={})", err, session_id);
                            break;
                        }
                    }
                }
            }
        }

        println!(
            "WS loop exiting (session_id={}, counted_viewer={})",
            session_id, counted_viewer
        );

        // ---- Disconnect: remove from active map and notify everyone
        if let Some(u) = this_user {
            USER_LOCATIONS.remove(&user_id);

            let msg = WsMsg::Disconnect {
                id: u.id.clone(),
                key: u.key.clone(),
            };
            let _ = TCP_BROADCAST.send(serde_json::to_string(&msg).unwrap());
        }

        if role == "viewer" && counted_viewer {
            if let Some(entry) = ACTIVE_SESSIONS.get(&session_id) {
                println!(
                    "Disconnect pre state (session_id={}, connections={}, last_seen={})",
                    session_id, entry.connections, entry.last_seen
                );
            } else {
                println!("Disconnect pre state missing (session_id={})", session_id);
            }

            let should_remove = if let Some(mut entry) = ACTIVE_SESSIONS.get_mut(&session_id) {
                if entry.connections > 1 {
                    entry.connections -= 1;
                    println!(
                        "Disconnect decremented (session_id={}, connections={})",
                        session_id, entry.connections
                    );
                    false
                } else {
                    true
                }
            } else {
                false
            };

            if should_remove {
                ACTIVE_SESSIONS.remove(&session_id);
                ACTIVE_USERS.fetch_sub(1, Ordering::SeqCst);
                println!("Disconnect removed session (session_id={})", session_id);
            }
            println!(
                "User disconnected, active users = {} (session_id={})",
                ACTIVE_USERS.load(Ordering::Relaxed),
                session_id
            );
        }
    }

    let influx_url = std::env::var("INFLUXDB_URL").expect("INFLUXDB_URL not set");
    let influx_org = std::env::var("INFLUXDB_ORG").expect("INFLUXDB_ORG not set");
    let influx_token = std::env::var("INFLUXDB_TOKEN")
        .expect("INFLUXDB_TOKEN not set")
        .trim()
        .to_string();

    let influx_bucket = std::env::var("INFLUXDB_BUCKET").expect("INFLUXDB_BUCKET not set");
    let influx = InfluxClient::new(influx_url, influx_org, influx_token);

    #[cfg(feature = "server")]
    {
        let influx = influx.clone();
        let bucket = influx_bucket.clone();

        tokio::spawn(async move {
            loop {
                match fetch_current_temperature(&influx, &bucket, "inside_temperature").await {
                    Some(temp) => {
                        *CURRENT_INSIDE_TEMPERATURE.write().unwrap() = Some(temp);
                        println!("Updated CURRENT_TEMPERATURE from Influx: {}", temp);
                    }
                    None => {
                        eprintln!("Failed to read inside_temperature from Influx");
                    }
                }

                match fetch_current_temperature(&influx, &bucket, "outside_temperature").await {
                    Some(temp) => {
                        *CURRENT_OUTSIDE_TEMPERATURE.write().unwrap() = Some(temp);
                        println!("Updated CURRENT_TEMPERATURE from Influx: {}", temp);
                    }
                    None => {
                        eprintln!("Failed to read inside_temperature from Influx");
                    }
                }

                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            }
        });
    }

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

    tokio::spawn(async {
        const SESSION_TTL_SECS: i64 = 15;
        let mut ticker = interval(Duration::from_secs(5));

        loop {
            ticker.tick().await;

            let now = chrono::Utc::now().timestamp();
            let mut removed = 0usize;
            let mut stale_keys = Vec::new();

            for entry in ACTIVE_SESSIONS.iter() {
                if now - entry.value().last_seen > SESSION_TTL_SECS {
                    stale_keys.push(entry.key().clone());
                }
            }

            for key in stale_keys {
                if ACTIVE_SESSIONS.remove(&key).is_some() {
                    removed += 1;
                }
            }

            if removed > 0 {
                ACTIVE_USERS.fetch_sub(removed, Ordering::SeqCst);
                println!(
                    "Pruned {} stale sessions, active users = {}",
                    removed,
                    ACTIVE_USERS.load(Ordering::Relaxed)
                );
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
        //.nest_service("/assets", ServeDir::new("public/assets"))
        .nest_service("/gallery-assets", ServeDir::new("gallery"))
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
