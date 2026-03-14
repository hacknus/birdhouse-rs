#[cfg(feature = "server")]
use chrono::Utc;
#[cfg(feature = "server")]
use dashmap::DashMap;
#[cfg(feature = "server")]
use once_cell::sync::Lazy;
#[cfg(feature = "server")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "server")]
use sha2::{Digest, Sha256};
#[cfg(feature = "server")]
use std::path::Path;
#[cfg(feature = "server")]
use std::sync::RwLock;
#[cfg(feature = "server")]
use uuid::Uuid;
#[cfg(feature = "server")]
use webauthn_rs::prelude::*;

#[cfg(feature = "server")]
const ADMIN_CREDENTIALS_FILE: &str = "data/admin_credentials.json";
#[cfg(feature = "server")]
const ADMIN_SESSION_TTL_SECS: i64 = 12 * 60 * 60;
#[cfg(feature = "server")]
const PASSKEY_FLOW_TTL_SECS: i64 = 5 * 60;
#[cfg(feature = "server")]
const LOGIN_RATE_LIMIT_WINDOW_SECS: i64 = 60;
#[cfg(feature = "server")]
const LOGIN_RATE_LIMIT_MAX_ATTEMPTS: usize = 5;

#[cfg(feature = "server")]
#[derive(Clone)]
struct AdminSession {
    last_seen: i64,
}

#[cfg(feature = "server")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AdminCredentials {
    #[serde(default)]
    username: String,
    email: String,
    password_salt: String,
    password_hash: String,
    #[serde(default)]
    user_uuid: String,
    #[serde(default)]
    passkey: Option<Passkey>,
    updated_at: i64,
}

#[cfg(feature = "server")]
#[derive(Debug, Clone, Deserialize)]
struct LegacyAdminCredentials {
    #[serde(default, alias = "ADMIN_USERNAME")]
    username: String,
    #[serde(default, alias = "ADMIN_EMAIL")]
    email: String,
    #[serde(default, alias = "ADMIN_PASSWORD")]
    password: String,
}

#[cfg(feature = "server")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminProfile {
    pub email: String,
    pub has_passkey: bool,
}

#[cfg(feature = "server")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasskeyBeginResult {
    pub flow_id: String,
    pub options_json: String,
}

#[cfg(feature = "server")]
struct PasskeyRegistrationFlow {
    state: PasskeyRegistration,
    session_token: String,
    expires_at: i64,
}

#[cfg(feature = "server")]
struct PasskeyAuthenticationFlow {
    state: PasskeyAuthentication,
    email: String,
    expires_at: i64,
}

#[cfg(feature = "server")]
static ADMIN_SESSIONS: Lazy<DashMap<String, AdminSession>> = Lazy::new(DashMap::new);
#[cfg(feature = "server")]
static ADMIN_CREDENTIALS_CACHE: Lazy<RwLock<Option<AdminCredentials>>> =
    Lazy::new(|| RwLock::new(None));
#[cfg(feature = "server")]
static PASSKEY_REGISTRATION_FLOWS: Lazy<DashMap<String, PasskeyRegistrationFlow>> =
    Lazy::new(DashMap::new);
#[cfg(feature = "server")]
static PASSKEY_AUTH_FLOWS: Lazy<DashMap<String, PasskeyAuthenticationFlow>> =
    Lazy::new(DashMap::new);
#[cfg(feature = "server")]
static LOGIN_ATTEMPTS: Lazy<DashMap<String, Vec<i64>>> = Lazy::new(DashMap::new);

#[cfg(feature = "server")]
static ADMIN_WEBAUTHN: Lazy<Result<Webauthn, String>> = Lazy::new(|| {
    fn normalize_rp_id(raw: &str) -> String {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return String::new();
        }

        if trimmed.contains("://") {
            if let Ok(url) = reqwest::Url::parse(trimmed) {
                if let Some(host) = url.host_str() {
                    return host.trim().trim_matches('.').to_string();
                }
            }
        }

        trimmed
            .split('/')
            .next()
            .unwrap_or(trimmed)
            .split(':')
            .next()
            .unwrap_or(trimmed)
            .trim()
            .trim_matches('.')
            .to_string()
    }

    let origin_raw = std::env::var("ADMIN_WEBAUTHN_ORIGIN").unwrap_or_else(|_| {
        let port = std::env::var("ADMIN_WEBAUTHN_PORT").unwrap_or_else(|_| "8080".to_string());
        format!("http://localhost:{}", port)
    });
    let origin = reqwest::Url::parse(&origin_raw)
        .map_err(|e| format!("Invalid ADMIN_WEBAUTHN_ORIGIN: {}", e))?;
    let origin_host = origin
        .host_str()
        .ok_or_else(|| "ADMIN_WEBAUTHN_ORIGIN is missing a valid host.".to_string())?
        .to_string();

    let configured_rp_id = std::env::var("ADMIN_WEBAUTHN_RP_ID")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(|v| normalize_rp_id(&v));
    let mut rp_id = configured_rp_id
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| origin_host.clone());
    if origin_host != rp_id && !origin_host.ends_with(&format!(".{}", rp_id)) {
        eprintln!(
            "ADMIN_WEBAUTHN_RP_ID='{}' is incompatible with origin host '{}'; falling back to '{}'",
            rp_id, origin_host, origin_host
        );
        rp_id = origin_host.clone();
    }

    let rp_name = std::env::var("ADMIN_WEBAUTHN_RP_NAME")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "Birdhouse Admin".to_string());

    WebauthnBuilder::new(&rp_id, &origin)
        .map_err(|e| format!("Failed to initialize WebAuthn builder: {}", e))?
        .rp_name(&rp_name)
        .build()
        .map_err(|e| format!("Failed to build WebAuthn config: {}", e))
});

#[cfg(feature = "server")]
fn admin_webauthn() -> Result<&'static Webauthn, String> {
    match &*ADMIN_WEBAUTHN {
        Ok(v) => Ok(v),
        Err(e) => Err(e.clone()),
    }
}

#[cfg(feature = "server")]
fn hash_secret(secret: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(b":");
    hasher.update(secret.as_bytes());
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(feature = "server")]
fn default_credentials() -> AdminCredentials {
    let email = std::env::var("ADMIN_EMAIL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| {
            std::env::var("ADMIN_USERNAME")
                .ok()
                .filter(|v| !v.trim().is_empty())
        })
        .unwrap_or_else(|| "admin@proton.me".to_string());
    let password = std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin".to_string());
    let password_salt = Uuid::new_v4().to_string();

    AdminCredentials {
        username: email.clone(),
        email,
        password_hash: hash_secret(&password, &password_salt),
        password_salt,
        user_uuid: Uuid::new_v4().to_string(),
        passkey: None,
        updated_at: Utc::now().timestamp(),
    }
}

#[cfg(feature = "server")]
fn write_credentials(credentials: &AdminCredentials) -> Result<(), String> {
    let path = Path::new(ADMIN_CREDENTIALS_FILE);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create admin credentials directory: {}", e))?;
    }

    let json = serde_json::to_string_pretty(credentials)
        .map_err(|e| format!("Failed to serialize admin credentials: {}", e))?;
    std::fs::write(path, json).map_err(|e| format!("Failed to write admin credentials: {}", e))?;
    Ok(())
}

#[cfg(feature = "server")]
fn read_or_create_credentials() -> Result<AdminCredentials, String> {
    let path = Path::new(ADMIN_CREDENTIALS_FILE);
    if !path.exists() {
        let defaults = default_credentials();
        write_credentials(&defaults)?;
        return Ok(defaults);
    }

    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read admin credentials: {}", e))?;
    let mut credentials = match serde_json::from_str::<AdminCredentials>(&text) {
        Ok(parsed) => parsed,
        Err(new_format_err) => {
            // Backward-compatible migration from a legacy plaintext format:
            // {
            //   "ADMIN_USERNAME": "...",
            //   "ADMIN_PASSWORD": "...",
            //   "ADMIN_EMAIL": "..."
            // }
            let legacy = serde_json::from_str::<LegacyAdminCredentials>(&text).map_err(|_| {
                format!("Failed to parse admin credentials JSON: {}", new_format_err)
            })?;

            let email = if !legacy.email.trim().is_empty() {
                legacy.email.trim().to_string()
            } else if !legacy.username.trim().is_empty() {
                legacy.username.trim().to_string()
            } else {
                std::env::var("ADMIN_EMAIL")
                    .ok()
                    .filter(|v| !v.trim().is_empty())
                    .or_else(|| {
                        std::env::var("ADMIN_USERNAME")
                            .ok()
                            .filter(|v| !v.trim().is_empty())
                    })
                    .unwrap_or_else(|| "admin".to_string())
            };
            let password = if legacy.password.is_empty() {
                std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin".to_string())
            } else {
                legacy.password
            };

            let migrated = AdminCredentials {
                username: email.clone(),
                email,
                password_salt: Uuid::new_v4().to_string(),
                password_hash: String::new(),
                user_uuid: Uuid::new_v4().to_string(),
                passkey: None,
                updated_at: Utc::now().timestamp(),
            };
            let migrated = AdminCredentials {
                password_hash: hash_secret(&password, &migrated.password_salt),
                ..migrated
            };
            write_credentials(&migrated)?;
            migrated
        }
    };

    if credentials.user_uuid.trim().is_empty() {
        credentials.user_uuid = Uuid::new_v4().to_string();
        write_credentials(&credentials)?;
    }
    if credentials.email.trim().is_empty() {
        credentials.email = credentials.username.trim().to_string();
        if credentials.email.is_empty() {
            credentials.email = "admin".to_string();
        }
        write_credentials(&credentials)?;
    }
    if credentials.username.trim().is_empty() || credentials.username != credentials.email {
        credentials.username = credentials.email.clone();
        write_credentials(&credentials)?;
    }

    Ok(credentials)
}

#[cfg(feature = "server")]
fn load_credentials() -> Result<AdminCredentials, String> {
    if let Ok(cache) = ADMIN_CREDENTIALS_CACHE.read() {
        if let Some(credentials) = cache.as_ref() {
            return Ok(credentials.clone());
        }
    }

    let credentials = read_or_create_credentials()?;
    if let Ok(mut cache) = ADMIN_CREDENTIALS_CACHE.write() {
        *cache = Some(credentials.clone());
    }
    Ok(credentials)
}

#[cfg(feature = "server")]
fn save_credentials(credentials: &AdminCredentials) -> Result<(), String> {
    write_credentials(credentials)?;
    if let Ok(mut cache) = ADMIN_CREDENTIALS_CACHE.write() {
        *cache = Some(credentials.clone());
    }
    Ok(())
}

#[cfg(feature = "server")]
fn prune_sessions(now: i64) {
    let mut stale = Vec::new();
    for entry in ADMIN_SESSIONS.iter() {
        if now - entry.value().last_seen > ADMIN_SESSION_TTL_SECS {
            stale.push(entry.key().clone());
        }
    }
    for token in stale {
        ADMIN_SESSIONS.remove(&token);
    }
}

#[cfg(feature = "server")]
fn prune_passkey_flows(now: i64) {
    let mut stale_reg = Vec::new();
    for entry in PASSKEY_REGISTRATION_FLOWS.iter() {
        if entry.value().expires_at <= now {
            stale_reg.push(entry.key().clone());
        }
    }
    for id in stale_reg {
        PASSKEY_REGISTRATION_FLOWS.remove(&id);
    }

    let mut stale_auth = Vec::new();
    for entry in PASSKEY_AUTH_FLOWS.iter() {
        if entry.value().expires_at <= now {
            stale_auth.push(entry.key().clone());
        }
    }
    for id in stale_auth {
        PASSKEY_AUTH_FLOWS.remove(&id);
    }
}

#[cfg(feature = "server")]
fn rate_limit_key(email: &str) -> String {
    email.trim().to_ascii_lowercase()
}

#[cfg(feature = "server")]
fn check_login_rate_limit(email: &str, now: i64) -> Result<(), String> {
    let key = rate_limit_key(email);
    let mut attempts = LOGIN_ATTEMPTS.entry(key).or_default();
    attempts.retain(|attempt| now - *attempt < LOGIN_RATE_LIMIT_WINDOW_SECS);
    if attempts.len() >= LOGIN_RATE_LIMIT_MAX_ATTEMPTS {
        return Err("Too many login attempts. Please wait a minute and try again.".to_string());
    }
    Ok(())
}

#[cfg(feature = "server")]
fn record_login_attempt(email: &str, now: i64) {
    let key = rate_limit_key(email);
    let mut attempts = LOGIN_ATTEMPTS.entry(key).or_default();
    attempts.retain(|attempt| now - *attempt < LOGIN_RATE_LIMIT_WINDOW_SECS);
    attempts.push(now);
}

#[cfg(feature = "server")]
fn clear_login_attempts(email: &str) {
    LOGIN_ATTEMPTS.remove(&rate_limit_key(email));
}

#[cfg(feature = "server")]
pub fn admin_login_password(email: &str, password: &str) -> Result<String, String> {
    let credentials = load_credentials()?;
    let now = Utc::now().timestamp();
    prune_sessions(now);
    prune_passkey_flows(now);
    check_login_rate_limit(email, now)?;

    if email.trim() != credentials.email {
        record_login_attempt(email, now);
        return Err("Invalid admin credentials.".to_string());
    }

    let password_ok =
        hash_secret(password, &credentials.password_salt) == credentials.password_hash;
    if !password_ok {
        record_login_attempt(email, now);
        return Err("Invalid admin credentials.".to_string());
    }

    clear_login_attempts(email);
    let token = Uuid::new_v4().to_string();
    ADMIN_SESSIONS.insert(token.clone(), AdminSession { last_seen: now });
    Ok(token)
}

#[cfg(feature = "server")]
pub fn admin_logout(token: &str) {
    ADMIN_SESSIONS.remove(token);
}

#[cfg(feature = "server")]
pub fn admin_validate_session(token: &str) -> bool {
    let token = token.trim();
    if token.is_empty() {
        return false;
    }

    let now = Utc::now().timestamp();
    prune_sessions(now);

    if let Some(mut entry) = ADMIN_SESSIONS.get_mut(token) {
        entry.last_seen = now;
        true
    } else {
        false
    }
}

#[cfg(feature = "server")]
pub fn admin_profile(token: &str) -> Result<AdminProfile, String> {
    if !admin_validate_session(token) {
        return Err("Unauthorized".to_string());
    }

    let credentials = load_credentials()?;
    Ok(AdminProfile {
        email: credentials.email,
        has_passkey: credentials.passkey.is_some(),
    })
}

#[cfg(feature = "server")]
pub fn admin_update_credentials(
    token: &str,
    email: String,
    new_password: Option<String>,
) -> Result<(), String> {
    if !admin_validate_session(token) {
        return Err("Unauthorized".to_string());
    }

    let email = email.trim().to_string();

    if email.is_empty() {
        return Err("Email cannot be empty.".to_string());
    }

    let mut credentials = load_credentials()?;
    credentials.username = email.clone();
    credentials.email = email;

    if let Some(password) = new_password.map(|v| v.trim().to_string()) {
        if !password.is_empty() {
            credentials.password_salt = Uuid::new_v4().to_string();
            credentials.password_hash = hash_secret(&password, &credentials.password_salt);
        }
    }

    credentials.updated_at = Utc::now().timestamp();
    save_credentials(&credentials)
}

#[cfg(feature = "server")]
pub fn admin_begin_passkey_registration(token: &str) -> Result<PasskeyBeginResult, String> {
    if !admin_validate_session(token) {
        return Err("Unauthorized".to_string());
    }

    let now = Utc::now().timestamp();
    prune_passkey_flows(now);

    let mut credentials = load_credentials()?;
    if credentials.user_uuid.trim().is_empty() {
        credentials.user_uuid = Uuid::new_v4().to_string();
        save_credentials(&credentials)?;
    }
    let user_uuid =
        Uuid::parse_str(&credentials.user_uuid).map_err(|e| format!("Invalid user uuid: {}", e))?;

    let (creation, state) = admin_webauthn()?
        .start_passkey_registration(user_uuid, &credentials.email, &credentials.email, None)
        .map_err(|e| format!("Failed to start passkey registration: {}", e))?;

    let flow_id = Uuid::new_v4().to_string();
    PASSKEY_REGISTRATION_FLOWS.insert(
        flow_id.clone(),
        PasskeyRegistrationFlow {
            state,
            session_token: token.to_string(),
            expires_at: now + PASSKEY_FLOW_TTL_SECS,
        },
    );

    let options_json = serde_json::to_string(&creation)
        .map_err(|e| format!("Failed to serialize passkey registration options: {}", e))?;
    Ok(PasskeyBeginResult {
        flow_id,
        options_json,
    })
}

#[cfg(feature = "server")]
pub fn admin_finish_passkey_registration(
    token: &str,
    flow_id: &str,
    credential_json: &str,
) -> Result<(), String> {
    if !admin_validate_session(token) {
        return Err("Unauthorized".to_string());
    }

    let now = Utc::now().timestamp();
    prune_passkey_flows(now);

    let (_, flow) = PASSKEY_REGISTRATION_FLOWS
        .remove(flow_id)
        .ok_or_else(|| "Passkey registration session expired.".to_string())?;
    if flow.expires_at <= now {
        return Err("Passkey registration session expired.".to_string());
    }
    if flow.session_token != token {
        return Err("Unauthorized".to_string());
    }

    let credential: RegisterPublicKeyCredential = serde_json::from_str(credential_json)
        .map_err(|e| format!("Invalid passkey registration payload: {}", e))?;
    let passkey = admin_webauthn()?
        .finish_passkey_registration(&credential, &flow.state)
        .map_err(|e| format!("Passkey registration failed: {}", e))?;

    let mut credentials = load_credentials()?;
    credentials.passkey = Some(passkey);
    credentials.updated_at = now;
    save_credentials(&credentials)
}

#[cfg(feature = "server")]
pub fn admin_begin_passkey_login(email: &str) -> Result<PasskeyBeginResult, String> {
    let now = Utc::now().timestamp();
    prune_sessions(now);
    prune_passkey_flows(now);
    check_login_rate_limit(email, now)?;

    let credentials = load_credentials()?;
    if email.trim() != credentials.email {
        record_login_attempt(email, now);
        return Err("Invalid admin credentials.".to_string());
    }
    let passkey = credentials.passkey.as_ref().ok_or_else(|| {
        record_login_attempt(email, now);
        "No passkey configured for this admin account.".to_string()
    })?;

    let (request, state) = admin_webauthn()?
        .start_passkey_authentication(std::slice::from_ref(passkey))
        .map_err(|e| format!("Failed to start passkey login: {}", e))?;

    clear_login_attempts(email);
    let flow_id = Uuid::new_v4().to_string();
    PASSKEY_AUTH_FLOWS.insert(
        flow_id.clone(),
        PasskeyAuthenticationFlow {
            state,
            email: credentials.email.clone(),
            expires_at: now + PASSKEY_FLOW_TTL_SECS,
        },
    );

    let options_json = serde_json::to_string(&request)
        .map_err(|e| format!("Failed to serialize passkey login options: {}", e))?;
    Ok(PasskeyBeginResult {
        flow_id,
        options_json,
    })
}

#[cfg(feature = "server")]
pub fn admin_finish_passkey_login(flow_id: &str, credential_json: &str) -> Result<String, String> {
    let now = Utc::now().timestamp();
    prune_sessions(now);
    prune_passkey_flows(now);

    let (_, flow) = PASSKEY_AUTH_FLOWS
        .remove(flow_id)
        .ok_or_else(|| "Passkey login session expired.".to_string())?;
    if flow.expires_at <= now {
        return Err("Passkey login session expired.".to_string());
    }

    let credential: PublicKeyCredential = serde_json::from_str(credential_json)
        .map_err(|e| format!("Invalid passkey login payload: {}", e))?;
    admin_webauthn()?
        .finish_passkey_authentication(&credential, &flow.state)
        .map_err(|e| format!("Passkey login failed: {}", e))?;

    let credentials = load_credentials()?;
    if credentials.email != flow.email {
        return Err("Invalid admin credentials.".to_string());
    }

    let token = Uuid::new_v4().to_string();
    ADMIN_SESSIONS.insert(token.clone(), AdminSession { last_seen: now });
    Ok(token)
}
