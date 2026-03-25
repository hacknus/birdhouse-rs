use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use gloo_net::http::Request;
#[cfg(target_arch = "wasm32")]
use js_sys::eval;
use serde::{Deserialize, Serialize};

const IR_LUX_THRESHOLD: f64 = 300.0;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct AdminProfileView {
    email: String,
    has_passkey: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct AdminGalleryImage {
    filename: String,
    url: String,
    thumbnail_url: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct AdminDeviceStatus {
    ir_enabled: bool,
    luminosity_lux: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct PasskeyBeginResultView {
    flow_id: String,
    options_json: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug, Deserialize)]
struct AdminPasswordLoginApiResponse {
    token: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug, Deserialize)]
struct AdminPasskeyLoginApiResponse {
    token: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug, Deserialize)]
struct AdminValidateSessionApiResponse {
    valid: bool,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug, Deserialize)]
struct AdminApiErrorResponse {
    error: String,
}

#[cfg(target_arch = "wasm32")]
fn read_admin_token_from_storage() -> Option<String> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    storage.get_item("birdhouse_admin_token").ok()?
}

#[cfg(target_arch = "wasm32")]
fn write_admin_token_to_storage(token: &str) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item("birdhouse_admin_token", token);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn clear_admin_token_from_storage() {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.remove_item("birdhouse_admin_token");
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn parse_admin_api_error(text: &str, status: u16) -> String {
    if let Ok(parsed) = serde_json::from_str::<AdminApiErrorResponse>(text) {
        if !parsed.error.trim().is_empty() {
            return parsed.error;
        }
    }
    if !text.trim().is_empty() {
        return text.to_string();
    }
    format!("Request failed with status {}", status)
}

#[cfg(target_arch = "wasm32")]
async fn admin_password_login_http(email: String, password: String) -> Result<String, String> {
    let body = serde_json::json!({
        "email": email,
        "password": password,
    })
    .to_string();

    let response = Request::post("/api/admin/password-login")
        .header("content-type", "application/json")
        .body(body)
        .map_err(|e| format!("Failed to build login request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Login request failed: {}", e))?;

    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if status >= 200 && status < 300 {
        let payload = serde_json::from_str::<AdminPasswordLoginApiResponse>(&text)
            .map_err(|e| format!("Invalid login response: {}", e))?;
        return Ok(payload.token);
    }

    Err(parse_admin_api_error(&text, status))
}

#[cfg(target_arch = "wasm32")]
async fn admin_validate_session_http(token: String) -> Result<bool, String> {
    let body = serde_json::json!({ "token": token }).to_string();

    let response = Request::post("/api/admin/validate-session")
        .header("content-type", "application/json")
        .body(body)
        .map_err(|e| format!("Failed to build session validation request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Session validation request failed: {}", e))?;

    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if status >= 200 && status < 300 {
        let payload = serde_json::from_str::<AdminValidateSessionApiResponse>(&text)
            .map_err(|e| format!("Invalid session validation response: {}", e))?;
        return Ok(payload.valid);
    }

    Err(parse_admin_api_error(&text, status))
}

#[cfg(target_arch = "wasm32")]
async fn admin_begin_passkey_login_http(email: String) -> Result<PasskeyBeginResultView, String> {
    let body = serde_json::json!({ "email": email }).to_string();
    let response = Request::post("/api/admin/passkey/begin-login")
        .header("content-type", "application/json")
        .body(body)
        .map_err(|e| format!("Failed to build passkey login request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Passkey login request failed: {}", e))?;

    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if status >= 200 && status < 300 {
        return serde_json::from_str::<PasskeyBeginResultView>(&text)
            .map_err(|e| format!("Invalid passkey login response: {}", e));
    }
    Err(parse_admin_api_error(&text, status))
}

#[cfg(target_arch = "wasm32")]
async fn admin_finish_passkey_login_http(
    flow_id: String,
    credential_json: String,
) -> Result<String, String> {
    let body = serde_json::json!({
        "flow_id": flow_id,
        "credential_json": credential_json
    })
    .to_string();
    let response = Request::post("/api/admin/passkey/finish-login")
        .header("content-type", "application/json")
        .body(body)
        .map_err(|e| format!("Failed to build passkey finish login request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Passkey finish login request failed: {}", e))?;

    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if status >= 200 && status < 300 {
        let payload = serde_json::from_str::<AdminPasskeyLoginApiResponse>(&text)
            .map_err(|e| format!("Invalid passkey login response: {}", e))?;
        return Ok(payload.token);
    }
    Err(parse_admin_api_error(&text, status))
}

#[cfg(target_arch = "wasm32")]
async fn admin_begin_passkey_registration_http(
    token: String,
) -> Result<PasskeyBeginResultView, String> {
    let body = serde_json::json!({ "token": token }).to_string();
    let response = Request::post("/api/admin/passkey/begin-registration")
        .header("content-type", "application/json")
        .body(body)
        .map_err(|e| format!("Failed to build passkey registration request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Passkey registration request failed: {}", e))?;

    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if status >= 200 && status < 300 {
        return serde_json::from_str::<PasskeyBeginResultView>(&text)
            .map_err(|e| format!("Invalid passkey registration response: {}", e));
    }
    Err(parse_admin_api_error(&text, status))
}

#[cfg(target_arch = "wasm32")]
async fn admin_finish_passkey_registration_http(
    token: String,
    flow_id: String,
    credential_json: String,
) -> Result<(), String> {
    let body = serde_json::json!({
        "token": token,
        "flow_id": flow_id,
        "credential_json": credential_json
    })
    .to_string();
    let response = Request::post("/api/admin/passkey/finish-registration")
        .header("content-type", "application/json")
        .body(body)
        .map_err(|e| format!("Failed to build passkey finish registration request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Passkey finish registration request failed: {}", e))?;

    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if status >= 200 && status < 300 {
        return Ok(());
    }
    Err(parse_admin_api_error(&text, status))
}

#[cfg(not(target_arch = "wasm32"))]
async fn admin_password_login_http(_email: String, _password: String) -> Result<String, String> {
    Err("Password login HTTP is only available in the browser.".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
async fn admin_validate_session_http(_token: String) -> Result<bool, String> {
    Ok(false)
}

#[cfg(not(target_arch = "wasm32"))]
async fn admin_begin_passkey_login_http(_email: String) -> Result<PasskeyBeginResultView, String> {
    Err("Passkey login HTTP is only available in the browser.".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
async fn admin_finish_passkey_login_http(
    _flow_id: String,
    _credential_json: String,
) -> Result<String, String> {
    Err("Passkey login HTTP is only available in the browser.".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
async fn admin_begin_passkey_registration_http(
    _token: String,
) -> Result<PasskeyBeginResultView, String> {
    Err("Passkey registration HTTP is only available in the browser.".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
async fn admin_finish_passkey_registration_http(
    _token: String,
    _flow_id: String,
    _credential_json: String,
) -> Result<(), String> {
    Err("Passkey registration HTTP is only available in the browser.".to_string())
}

#[cfg(target_arch = "wasm32")]
fn passkey_domain_hint() -> Option<String> {
    let window = web_sys::window()?;
    let hostname = window.location().hostname().ok()?;
    if hostname.eq_ignore_ascii_case("localhost") {
        return None;
    }
    if hostname.parse::<std::net::Ipv4Addr>().is_ok() || hostname.contains(':') {
        return Some(
            "Passkeys on Safari require a domain host. Open the app using http://localhost:<port>/admin (not an IP address).".to_string(),
        );
    }
    None
}

#[cfg(target_arch = "wasm32")]
async fn run_webauthn_create(options_json: &str) -> Result<String, String> {
    if let Some(hint) = passkey_domain_hint() {
        return Err(hint);
    }

    let options_literal = serde_json::to_string(options_json)
        .map_err(|e| format!("Failed to encode passkey options: {}", e))?;
    let script = format!(
        r#"(async () => {{
            if (!window.isSecureContext) {{
                throw new Error("Passkeys require a secure context (HTTPS or localhost).");
            }}
            if (!navigator.credentials || !navigator.credentials.create) {{
                throw new Error("Passkey API not available in this browser.");
            }}

            const options = JSON.parse({options_literal});
            const publicKey = options.publicKey ? options.publicKey : options;

            const fromB64Url = (v) => {{
                const b64 = String(v).replace(/-/g, "+").replace(/_/g, "/");
                const padded = b64 + "=".repeat((4 - (b64.length % 4 || 4)) % 4);
                const raw = atob(padded);
                const out = new Uint8Array(raw.length);
                for (let i = 0; i < raw.length; i++) out[i] = raw.charCodeAt(i);
                return out;
            }};

            const toB64Url = (buf) => {{
                if (!buf) return null;
                const bytes = new Uint8Array(buf);
                let binary = "";
                for (const b of bytes) binary += String.fromCharCode(b);
                return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/g, "");
            }};

            publicKey.challenge = fromB64Url(publicKey.challenge);
            if (publicKey.user && typeof publicKey.user.id === "string") {{
                publicKey.user.id = fromB64Url(publicKey.user.id);
            }}
            if (Array.isArray(publicKey.excludeCredentials)) {{
                publicKey.excludeCredentials = publicKey.excludeCredentials.map((c) => {{
                    const next = {{ ...c }};
                    if (typeof next.id === "string") next.id = fromB64Url(next.id);
                    return next;
                }});
            }}

            const credential = await navigator.credentials.create({{ publicKey }});
            if (!credential) throw new Error("No passkey credential returned.");

            const response = credential.response;
            const out = {{
                id: credential.id,
                rawId: toB64Url(credential.rawId),
                type: credential.type,
                response: {{
                    attestationObject: toB64Url(response.attestationObject),
                    clientDataJSON: toB64Url(response.clientDataJSON),
                }},
                clientExtensionResults: credential.getClientExtensionResults ? credential.getClientExtensionResults() : {{}},
                authenticatorAttachment: credential.authenticatorAttachment ?? null,
            }};
            if (response.getTransports) {{
                out.response.transports = response.getTransports();
            }}

            return JSON.stringify(out);
        }})()"#
    );

    let promise_value = eval(&script).map_err(|e| format!("Passkey script error: {:?}", e))?;
    let promise = promise_value
        .dyn_into::<js_sys::Promise>()
        .map_err(|_| "Passkey script did not return a Promise.".to_string())?;
    let result = JsFuture::from(promise)
        .await
        .map_err(|e| {
            let raw = format!("{:?}", e);
            if raw.contains("SecurityError") {
                "Passkey registration blocked by browser security policy. Use http://localhost:<port>/admin and ensure ADMIN_WEBAUTHN_ORIGIN uses the same host+port.".to_string()
            } else {
                format!("Passkey registration failed: {}", raw)
            }
        })?;
    result
        .as_string()
        .ok_or_else(|| "Passkey registration returned invalid payload.".to_string())
}

#[cfg(target_arch = "wasm32")]
async fn run_webauthn_get(options_json: &str) -> Result<String, String> {
    if let Some(hint) = passkey_domain_hint() {
        return Err(hint);
    }

    let options_literal = serde_json::to_string(options_json)
        .map_err(|e| format!("Failed to encode passkey options: {}", e))?;
    let script = format!(
        r#"(async () => {{
            if (!window.isSecureContext) {{
                throw new Error("Passkeys require a secure context (HTTPS or localhost).");
            }}
            if (!navigator.credentials || !navigator.credentials.get) {{
                throw new Error("Passkey API not available in this browser.");
            }}

            const options = JSON.parse({options_literal});
            const publicKey = options.publicKey ? options.publicKey : options;

            const fromB64Url = (v) => {{
                const b64 = String(v).replace(/-/g, "+").replace(/_/g, "/");
                const padded = b64 + "=".repeat((4 - (b64.length % 4 || 4)) % 4);
                const raw = atob(padded);
                const out = new Uint8Array(raw.length);
                for (let i = 0; i < raw.length; i++) out[i] = raw.charCodeAt(i);
                return out;
            }};

            const toB64Url = (buf) => {{
                if (!buf) return null;
                const bytes = new Uint8Array(buf);
                let binary = "";
                for (const b of bytes) binary += String.fromCharCode(b);
                return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/g, "");
            }};

            publicKey.challenge = fromB64Url(publicKey.challenge);
            if (Array.isArray(publicKey.allowCredentials)) {{
                publicKey.allowCredentials = publicKey.allowCredentials.map((c) => {{
                    const next = {{ ...c }};
                    if (typeof next.id === "string") next.id = fromB64Url(next.id);
                    return next;
                }});
            }}

            const credential = await navigator.credentials.get({{ publicKey }});
            if (!credential) throw new Error("No passkey credential returned.");

            const response = credential.response;
            const out = {{
                id: credential.id,
                rawId: toB64Url(credential.rawId),
                type: credential.type,
                response: {{
                    authenticatorData: toB64Url(response.authenticatorData),
                    clientDataJSON: toB64Url(response.clientDataJSON),
                    signature: toB64Url(response.signature),
                    userHandle: response.userHandle ? toB64Url(response.userHandle) : null,
                }},
                clientExtensionResults: credential.getClientExtensionResults ? credential.getClientExtensionResults() : {{}},
                authenticatorAttachment: credential.authenticatorAttachment ?? null,
            }};
            return JSON.stringify(out);
        }})()"#
    );

    let promise_value = eval(&script).map_err(|e| format!("Passkey script error: {:?}", e))?;
    let promise = promise_value
        .dyn_into::<js_sys::Promise>()
        .map_err(|_| "Passkey script did not return a Promise.".to_string())?;
    let result = JsFuture::from(promise)
        .await
        .map_err(|e| {
            let raw = format!("{:?}", e);
            if raw.contains("SecurityError") {
                "Passkey login blocked by browser security policy. Use http://localhost:<port>/admin and ensure ADMIN_WEBAUTHN_ORIGIN uses the same host+port.".to_string()
            } else {
                format!("Passkey login failed: {}", raw)
            }
        })?;
    result
        .as_string()
        .ok_or_else(|| "Passkey login returned invalid payload.".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
async fn run_webauthn_create(_options_json: &str) -> Result<String, String> {
    Err("Passkey operations require a browser client.".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
async fn run_webauthn_get(_options_json: &str) -> Result<String, String> {
    Err("Passkey operations require a browser client.".to_string())
}

#[server]
async fn admin_login_password_server(
    email: String,
    password: String,
) -> Result<String, ServerFnError> {
    #[cfg(feature = "server")]
    {
        crate::admin::admin_login_password(&email, &password).map_err(ServerFnError::new)
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Not running on server"))
    }
}

#[server]
async fn admin_begin_passkey_login_server(
    email: String,
) -> Result<PasskeyBeginResultView, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let result = crate::admin::admin_begin_passkey_login(&email).map_err(ServerFnError::new)?;
        Ok(PasskeyBeginResultView {
            flow_id: result.flow_id,
            options_json: result.options_json,
        })
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Not running on server"))
    }
}

#[server]
async fn admin_finish_passkey_login_server(
    flow_id: String,
    credential_json: String,
) -> Result<String, ServerFnError> {
    #[cfg(feature = "server")]
    {
        crate::admin::admin_finish_passkey_login(&flow_id, &credential_json)
            .map_err(ServerFnError::new)
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Not running on server"))
    }
}

#[server]
async fn admin_begin_passkey_registration_server(
    token: String,
) -> Result<PasskeyBeginResultView, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let result =
            crate::admin::admin_begin_passkey_registration(&token).map_err(ServerFnError::new)?;
        Ok(PasskeyBeginResultView {
            flow_id: result.flow_id,
            options_json: result.options_json,
        })
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Not running on server"))
    }
}

#[server]
async fn admin_finish_passkey_registration_server(
    token: String,
    flow_id: String,
    credential_json: String,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        crate::admin::admin_finish_passkey_registration(&token, &flow_id, &credential_json)
            .map_err(ServerFnError::new)
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Not running on server"))
    }
}

#[server]
async fn admin_validate_session_server(token: String) -> Result<bool, ServerFnError> {
    #[cfg(feature = "server")]
    {
        Ok(crate::admin::admin_validate_session(&token))
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Not running on server"))
    }
}

#[server]
async fn admin_logout_server(token: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        crate::admin::admin_logout(&token);
        Ok(())
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Not running on server"))
    }
}

#[server]
async fn admin_get_profile_server(token: String) -> Result<AdminProfileView, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let profile = crate::admin::admin_profile(&token).map_err(ServerFnError::new)?;
        Ok(AdminProfileView {
            email: profile.email,
            has_passkey: profile.has_passkey,
        })
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Not running on server"))
    }
}

#[server]
async fn admin_update_credentials_server(
    token: String,
    email: String,
    new_password: String,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        crate::admin::admin_update_credentials(
            &token,
            email,
            if new_password.trim().is_empty() {
                None
            } else {
                Some(new_password)
            },
        )
        .map_err(ServerFnError::new)
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Not running on server"))
    }
}

#[server]
async fn admin_list_gallery_images_server(
    token: String,
) -> Result<Vec<AdminGalleryImage>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use std::fs;

        if !crate::admin::admin_validate_session(&token) {
            return Err(ServerFnError::new("Unauthorized"));
        }

        let local_cache = "./gallery";
        fs::create_dir_all(local_cache)
            .map_err(|e| ServerFnError::new(format!("Failed to create gallery dir: {}", e)))?;

        let entries = fs::read_dir(local_cache)
            .map_err(|e| ServerFnError::new(format!("Failed to read directory: {}", e)))?;

        let mut images = Vec::new();
        for entry in entries {
            let entry =
                entry.map_err(|e| ServerFnError::new(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();
            let metadata = entry
                .metadata()
                .map_err(|e| ServerFnError::new(format!("Failed to stat entry: {}", e)))?;
            if metadata.len() == 0 {
                continue;
            }
            let ext = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            if !matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "gif" | "webp") {
                continue;
            }

            if let Some(filename) = path.file_name().and_then(|v| v.to_str()) {
                images.push(AdminGalleryImage {
                    filename: filename.to_string(),
                    url: format!("/gallery-assets/{}", filename),
                    thumbnail_url: format!("/gallery-thumbnails/{}", filename),
                });
            }
        }

        images.sort_by(|a, b| b.filename.cmp(&a.filename));
        Ok(images)
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Not running on server"))
    }
}

#[server]
async fn admin_delete_gallery_image_server(
    token: String,
    filename: String,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        use std::path::PathBuf;
        use tokio::fs;

        if !crate::admin::admin_validate_session(&token) {
            return Err(ServerFnError::new("Unauthorized"));
        }

        let safe_filename: String = filename
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
            .collect();
        if safe_filename.is_empty() || safe_filename != filename {
            return Err(ServerFnError::new("Invalid filename"));
        }

        let image_path = PathBuf::from("./gallery").join(&safe_filename);
        fs::remove_file(&image_path)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to remove image: {}", e)))?;

        let thumb_path = PathBuf::from("./gallery-thumbs").join(format!("{}.jpg", safe_filename));
        let _ = fs::remove_file(thumb_path).await;
        Ok(())
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Not running on server"))
    }
}

#[server]
async fn admin_upload_gallery_image_server(
    token: String,
    filename: String,
    bytes: Vec<u8>,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        use std::path::{Path, PathBuf};
        use tokio::fs;

        if !crate::admin::admin_validate_session(&token) {
            return Err(ServerFnError::new("Unauthorized"));
        }

        let safe_filename: String = filename
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
            .collect();
        if safe_filename.is_empty() || safe_filename != filename {
            return Err(ServerFnError::new("Invalid filename"));
        }

        let extension = Path::new(&safe_filename)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !matches!(extension.as_str(), "jpg" | "jpeg" | "png" | "gif" | "webp") {
            return Err(ServerFnError::new(
                "Invalid file type (allowed: jpg, jpeg, png, gif, webp)",
            ));
        }
        if bytes.is_empty() {
            return Err(ServerFnError::new("Empty upload"));
        }
        if bytes.len() > 20 * 1024 * 1024 {
            return Err(ServerFnError::new("File too large (max 20 MB)"));
        }

        fs::create_dir_all("./gallery")
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to create gallery dir: {}", e)))?;

        let path = format!("./gallery/{}", safe_filename);
        fs::write(&path, bytes)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to write image: {}", e)))?;

        let thumb_path = PathBuf::from("./gallery-thumbs").join(format!("{}.jpg", safe_filename));
        let _ = fs::remove_file(thumb_path).await;

        Ok(())
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Not running on server"))
    }
}

#[server]
async fn admin_get_device_status_server(token: String) -> Result<AdminDeviceStatus, ServerFnError> {
    #[cfg(feature = "server")]
    {
        if !crate::admin::admin_validate_session(&token) {
            return Err(ServerFnError::new("Unauthorized"));
        }

        let response = crate::tcp_client::send_command("[CMD] GET IR STATE")
            .await
            .map_err(ServerFnError::new)?;
        let payload = response.to_uppercase();

        let mut ir_enabled = None;
        for raw_line in payload.lines() {
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }

            if line.contains("IR STATE IS ON") {
                ir_enabled = Some(true);
                break;
            }

            if line.contains("IR STATE IS OFF") {
                ir_enabled = Some(false);
                break;
            }
        }

        let ir_enabled = ir_enabled.ok_or_else(|| {
            ServerFnError::new(format!(
                "Unexpected IR state response from TCP: {}",
                response
            ))
        })?;

        let luminosity_lux = {
            let lock = crate::CURRENT_LUMINOSITY
                .read()
                .map_err(|_| ServerFnError::new("Luminosity lock poisoned"))?;
            *lock
        };

        Ok(AdminDeviceStatus {
            ir_enabled,
            luminosity_lux,
        })
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Not running on server"))
    }
}

#[server]
async fn admin_toggle_ir_led_server(token: String, enabled: bool) -> Result<bool, ServerFnError> {
    #[cfg(feature = "server")]
    {
        if !crate::admin::admin_validate_session(&token) {
            return Err(ServerFnError::new("Unauthorized"));
        }

        let cmd = if enabled {
            "[CMD] IR ON"
        } else {
            "[CMD] IR OFF"
        };
        crate::tcp_client::send_command(cmd)
            .await
            .map(|_| enabled)
            .map_err(ServerFnError::new)
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Not running on server"))
    }
}

#[server]
async fn admin_save_image_server(token: String) -> Result<String, ServerFnError> {
    #[cfg(feature = "server")]
    {
        if !crate::admin::admin_validate_session(&token) {
            return Err(ServerFnError::new("Unauthorized"));
        }

        crate::tcp_client::send_command("[CMD] save image")
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to save image: {}", e)))?;

        Ok("Image saved successfully.".to_string())
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Not running on server"))
    }
}

#[component]
pub fn Admin() -> Element {
    #[cfg(target_arch = "wasm32")]
    let mut bootstrapped = use_signal(|| false);
    let mut auth_loading = use_signal(|| true);
    let mut is_authenticated = use_signal(|| false);
    let mut admin_token = use_signal(|| None::<String>);
    let mut status = use_signal(|| None::<String>);

    let mut login_email = use_signal(String::new);
    let mut login_password = use_signal(String::new);
    let mut login_busy = use_signal(|| false);
    let mut passkey_login_busy = use_signal(|| false);
    let mut login_feedback = use_signal(|| None::<String>);

    let mut profile_refresh = use_signal(|| 0u64);
    let mut gallery_refresh = use_signal(|| 0u64);
    let mut device_refresh = use_signal(|| 0u64);

    let mut settings_email = use_signal(String::new);
    let mut settings_new_password = use_signal(String::new);
    let mut settings_busy = use_signal(|| false);
    let mut passkey_registration_busy = use_signal(|| false);
    let mut admin_ir_enabled = use_signal(|| false);
    let mut admin_ir_busy = use_signal(|| false);
    let mut admin_ir_request_id = use_signal(|| 0u64);
    let mut admin_save_busy = use_signal(|| false);

    let mut upload_filename = use_signal(|| None::<String>);
    let mut upload_bytes = use_signal(|| None::<Vec<u8>>);
    let mut upload_busy = use_signal(|| false);

    let profile_resource = use_resource(move || {
        let _ = profile_refresh();
        let token = admin_token();
        async move {
            if let Some(token) = token {
                admin_get_profile_server(token).await.ok()
            } else {
                None
            }
        }
    });

    let gallery_resource = use_resource(move || {
        let _ = gallery_refresh();
        let token = admin_token();
        async move {
            if let Some(token) = token {
                admin_list_gallery_images_server(token)
                    .await
                    .map(Some)
                    .map_err(|e| e.to_string())
            } else {
                Ok(None)
            }
        }
    });

    let device_resource = use_resource(move || {
        let _ = device_refresh();
        let token = admin_token();
        async move {
            if let Some(token) = token {
                admin_get_device_status_server(token)
                    .await
                    .map(Some)
                    .map_err(|e| e.to_string())
            } else {
                Ok(None)
            }
        }
    });

    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        if bootstrapped() {
            return;
        }
        bootstrapped.set(true);
        spawn(async move {
            let mut is_valid = false;
            if let Some(stored) = read_admin_token_from_storage() {
                match admin_validate_session_http(stored.clone()).await {
                    Ok(true) => {
                        admin_token.set(Some(stored));
                        is_authenticated.set(true);
                        profile_refresh += 1;
                        gallery_refresh += 1;
                        device_refresh += 1;
                        is_valid = true;
                    }
                    _ => clear_admin_token_from_storage(),
                }
            }
            if !is_valid {
                is_authenticated.set(false);
                admin_token.set(None);
            }
            auth_loading.set(false);
        });
    });

    let profile_snapshot = profile_resource
        .read()
        .as_ref()
        .and_then(|v| v.as_ref().cloned());
    let profile_snapshot_for_effect = profile_snapshot.clone();
    use_effect(move || {
        if let Some(profile) = profile_snapshot_for_effect.clone() {
            if settings_email() != profile.email {
                settings_email.set(profile.email);
            }
        }
    });

    let device_snapshot = device_resource
        .read()
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .and_then(|value| value.clone());
    let device_snapshot_for_effect = device_snapshot.clone();
    use_effect(move || {
        if let Some(device_status) = device_snapshot_for_effect.clone() {
            admin_ir_enabled.set(device_status.ir_enabled);
        }
    });

    let mut logout = move || {
        let token = admin_token();
        admin_token.set(None);
        is_authenticated.set(false);
        status.set(Some("Logged out.".to_string()));
        #[cfg(target_arch = "wasm32")]
        clear_admin_token_from_storage();
        if let Some(token) = token {
            spawn(async move {
                let _ = admin_logout_server(token).await;
            });
        }
    };

    let mut handle_unauthorized = move || {
        status.set(Some(
            "Admin session expired. Please log in again.".to_string(),
        ));
        logout();
    };

    rsx! {
        section {
            class: "min-h-screen w-full bg-slate-900 text-white px-4 py-10",
            div {
                class: "mx-auto w-full max-w-6xl space-y-6",
                h1 { class: "text-3xl font-semibold", "Admin" }
                p { class: "text-slate-300", "Manage admin credentials and gallery content." }

                if auth_loading() {
                    div { class: "rounded-xl border border-slate-700 bg-slate-800 p-6", "Checking admin session..." }
                } else if !is_authenticated() {
                    div {
                        class: "rounded-xl border border-slate-700 bg-slate-800 p-6 max-w-xl space-y-4",
                        h2 { class: "text-xl font-medium", "Login" }
                        p { class: "text-slate-300 text-sm", "Sign in with email/password, or use passkey as an alternative." }
                        if let Some(msg) = login_feedback() {
                            p { class: "text-sm text-slate-200", "{msg}" }
                        }
                        form {
                            class: "space-y-3",
                            onsubmit: move |evt| {
                                evt.prevent_default();
                                if login_busy() || passkey_login_busy() {
                                    return;
                                }

                                let email = login_email().trim().to_string();
                                let password = login_password().to_string();
                                if email.is_empty() || password.is_empty() {
                                    let msg = "Please provide email and password.".to_string();
                                    status.set(Some(msg.clone()));
                                    login_feedback.set(Some(msg));
                                    return;
                                }

                                login_busy.set(true);
                                let start_msg = "Signing in with password...".to_string();
                                status.set(None);
                                login_feedback.set(Some(start_msg));
                                spawn(async move {
                                    match admin_password_login_http(email, password).await {
                                        Ok(token) => {
                                            #[cfg(target_arch = "wasm32")]
                                            write_admin_token_to_storage(&token);
                                            admin_token.set(Some(token));
                                            is_authenticated.set(true);
                                            login_password.set(String::new());
                                            profile_refresh += 1;
                                            gallery_refresh += 1;
                                            device_refresh += 1;
                                            status.set(Some("Admin login successful.".to_string()));
                                            login_feedback.set(None);
                                        }
                                        Err(err) => {
                                            let msg = format!("Login failed: {}", err);
                                            status.set(Some(msg.clone()));
                                            login_feedback.set(Some(msg));
                                        }
                                    }
                                    login_busy.set(false);
                                });
                            },
                            input {
                                r#type: "email",
                                placeholder: "Email",
                                value: login_email(),
                                class: "w-full rounded-md bg-white text-black px-3 py-2",
                                oninput: move |evt| login_email.set(evt.value()),
                            }
                            input {
                                r#type: "password",
                                placeholder: "Password",
                                value: login_password(),
                                class: "w-full rounded-md bg-white text-black px-3 py-2",
                                oninput: move |evt| login_password.set(evt.value()),
                            }
                            button {
                                r#type: "submit",
                                class: format!(
                                    "rounded-md px-4 py-2 font-medium {}",
                                    if login_busy() {
                                        "bg-slate-500 text-white"
                                    } else {
                                        "bg-emerald-500 hover:bg-emerald-600 text-white"
                                    }
                                ),
                                if login_busy() { "Signing in..." } else { "Sign in with Password" }
                            }
                        }

                        div { class: "flex flex-wrap gap-3",

                            button {
                                r#type: "button",
                                class: format!(
                                    "rounded-md px-4 py-2 font-medium {}",
                                    if passkey_login_busy() {
                                        "bg-slate-500 text-white"
                                    } else {
                                        "bg-blue-500 hover:bg-blue-600 text-white"
                                    }
                                ),
                                onclick: move |_| {
                                    if passkey_login_busy() || login_busy() {
                                        return;
                                    }
                                    let email = login_email().trim().to_string();
                                    if email.is_empty() {
                                        let msg = "Enter your email before using passkey login.".to_string();
                                        status.set(Some(msg.clone()));
                                        login_feedback.set(Some(msg));
                                        return;
                                    }

                                    passkey_login_busy.set(true);
                                    let start_msg = "Waiting for passkey...".to_string();
                                    status.set(None);
                                    login_feedback.set(Some(start_msg));
                                    spawn(async move {
                                        let result = async {
                                            let begin = admin_begin_passkey_login_http(email).await
                                                .map_err(|e| e.to_string())?;
                                            let credential_json = run_webauthn_get(&begin.options_json).await?;
                                            let token = admin_finish_passkey_login_http(begin.flow_id, credential_json).await
                                                .map_err(|e| e.to_string())?;
                                            Ok::<String, String>(token)
                                        }.await;

                                        match result {
                                            Ok(token) => {
                                                #[cfg(target_arch = "wasm32")]
                                                write_admin_token_to_storage(&token);
                                                admin_token.set(Some(token));
                                                is_authenticated.set(true);
                                                profile_refresh += 1;
                                                gallery_refresh += 1;
                                                device_refresh += 1;
                                                status.set(Some("Passkey login successful.".to_string()));
                                                login_feedback.set(None);
                                            }
                                            Err(err) => {
                                                let msg = format!("Passkey login failed: {}", err);
                                                status.set(Some(msg.clone()));
                                                login_feedback.set(Some(msg));
                                            }
                                        }
                                        passkey_login_busy.set(false);
                                    });
                                },
                                if passkey_login_busy() { "Waiting for Passkey..." } else { "Sign in with Passkey" }
                            }
                        }
                    }
                } else {
                    div { class: "grid grid-cols-1 xl:grid-cols-2 gap-6",
                        div {
                            class: "rounded-xl border border-slate-700 bg-slate-800 p-6 space-y-4",
                            div { class: "flex items-center justify-between",
                                h2 { class: "text-xl font-medium", "Admin Credentials" }
                                button {
                                    class: "rounded-md px-3 py-1 bg-slate-600 hover:bg-slate-500 text-white",
                                    onclick: move |_| logout(),
                                    "Logout"
                                }
                            }
                            p { class: "text-slate-300 text-sm", "Update email. Leave password empty to keep current password." }
                            input {
                                r#type: "email",
                                placeholder: "Email",
                                value: settings_email(),
                                class: "w-full rounded-md bg-white text-black px-3 py-2",
                                oninput: move |evt| settings_email.set(evt.value()),
                            }
                            input {
                                r#type: "password",
                                placeholder: "New Password (optional)",
                                value: settings_new_password(),
                                class: "w-full rounded-md bg-white text-black px-3 py-2",
                                oninput: move |evt| settings_new_password.set(evt.value()),
                            }
                            button {
                                r#type: "button",
                                class: format!(
                                    "rounded-md px-4 py-2 font-medium {}",
                                    if settings_busy() {
                                        "bg-slate-500 text-white"
                                    } else {
                                        "bg-emerald-500 hover:bg-emerald-600 text-white"
                                    }
                                ),
                                disabled: settings_busy() || passkey_registration_busy(),
                                onclick: move |_| {
                                    if settings_busy() || passkey_registration_busy() {
                                        return;
                                    }
                                    let Some(token) = admin_token() else {
                                        handle_unauthorized();
                                        return;
                                    };

                                    settings_busy.set(true);
                                    status.set(None);
                                    let email = settings_email().trim().to_string();
                                    let new_password = settings_new_password().to_string();

                                    spawn(async move {
                                        match admin_update_credentials_server(token, email, new_password).await {
                                            Ok(()) => {
                                                settings_new_password.set(String::new());
                                                profile_refresh += 1;
                                                status.set(Some("Admin credentials updated.".to_string()));
                                            }
                                            Err(err) => {
                                                let text = err.to_string();
                                                if text.contains("Unauthorized") {
                                                    handle_unauthorized();
                                                } else {
                                                    status.set(Some(format!("Update failed: {}", text)));
                                                }
                                            }
                                        }
                                        settings_busy.set(false);
                                    });
                                },
                                if settings_busy() { "Saving..." } else { "Save Settings" }
                            }

                            hr { class: "border-slate-700" }
                            p {
                                class: "text-slate-300 text-sm",
                                if profile_snapshot.as_ref().map(|p| p.has_passkey).unwrap_or(false) {
                                    "Passkey is configured for this account."
                                } else {
                                    "No passkey configured yet."
                                }
                            }
                            button {
                                r#type: "button",
                                class: format!(
                                    "rounded-md px-4 py-2 font-medium {}",
                                    if passkey_registration_busy() {
                                        "bg-slate-500 text-white"
                                    } else {
                                        "bg-blue-500 hover:bg-blue-600 text-white"
                                    }
                                ),
                                disabled: passkey_registration_busy() || settings_busy(),
                                onclick: move |_| {
                                    if passkey_registration_busy() || settings_busy() {
                                        return;
                                    }
                                    let Some(token) = admin_token() else {
                                        handle_unauthorized();
                                        return;
                                    };

                                    passkey_registration_busy.set(true);
                                    status.set(None);
                                    spawn(async move {
                                        let result = async {
                                            let begin = admin_begin_passkey_registration_http(token.clone()).await
                                                .map_err(|e| e.to_string())?;
                                            let credential_json = run_webauthn_create(&begin.options_json).await?;
                                            admin_finish_passkey_registration_http(token, begin.flow_id, credential_json).await
                                                .map_err(|e| e.to_string())?;
                                            Ok::<(), String>(())
                                        }.await;

                                        match result {
                                            Ok(()) => {
                                                profile_refresh += 1;
                                                status.set(Some("Passkey registered successfully.".to_string()));
                                            }
                                            Err(err) => {
                                                if err.contains("Unauthorized") {
                                                    handle_unauthorized();
                                                } else {
                                                    status.set(Some(format!("Passkey setup failed: {}", err)));
                                                }
                                            }
                                        }
                                        passkey_registration_busy.set(false);
                                    });
                                },
                                if passkey_registration_busy() { "Waiting for Passkey..." } else { "Register / Replace Passkey" }
                            }
                        }

                        div {
                            class: "rounded-xl border border-slate-700 bg-slate-800 p-6 space-y-4",
                            h2 { class: "text-xl font-medium", "Device Controls" }
                            if let Some(Ok(Some(device_status))) = device_resource.read().as_ref() {
                                {
                                    let lux_text = device_status
                                        .luminosity_lux
                                        .map(|lux| format!("{lux:.0} lux"))
                                        .unwrap_or_else(|| "unavailable".to_string());
                                    rsx! {
                                        p {
                                            class: "text-slate-300 text-sm",
                                            "Current luminosity: {lux_text}. Public IR control is limited below {IR_LUX_THRESHOLD:.0} lux, but these admin controls always remain available."
                                        }
                                    }
                                }
                            } else if let Some(Err(err)) = device_resource.read().as_ref() {
                                p { class: "text-sm text-red-300 break-all", "Failed to load device status: {err}" }
                            } else {
                                p { class: "text-slate-300 text-sm", "Loading device status..." }
                            }
                            div { class: "flex flex-wrap items-center gap-4",
                                div { class: "flex items-center gap-2",
                                    label {
                                        class: "text-white font-small whitespace-nowrap",
                                        "Light"
                                    }
                                    button {
                                        r#type: "button",
                                        class: format!(
                                            "relative inline-flex h-6 w-12 items-center rounded-full transition-colors {}",
                                            if admin_ir_enabled() {
                                                "bg-blue-500"
                                            } else {
                                                "bg-gray-600"
                                            }
                                        ),
                                        disabled: admin_ir_busy(),
                                        title: if admin_ir_busy() { "Updating IR LED..." } else { "toggle IR LED" },
                                        onclick: move |_| {
                                            if admin_ir_busy() {
                                                return;
                                            }
                                            let Some(token) = admin_token() else {
                                                handle_unauthorized();
                                                return;
                                            };

                                            admin_ir_busy.set(true);
                                            status.set(None);
                                            let previous_state = admin_ir_enabled();
                                            let next_state = !admin_ir_enabled();
                                            let request_id = admin_ir_request_id() + 1;
                                            admin_ir_request_id.set(request_id);
                                            admin_ir_enabled.set(next_state);

                                            let mut admin_ir_request_id_timeout = admin_ir_request_id;
                                            let mut admin_ir_enabled_timeout = admin_ir_enabled;
                                            let mut admin_ir_busy_timeout = admin_ir_busy;
                                            spawn(async move {
                                                #[cfg(target_arch = "wasm32")]
                                                gloo_timers::future::sleep(std::time::Duration::from_secs(5)).await;

                                                if admin_ir_request_id_timeout() == request_id {
                                                    admin_ir_enabled_timeout.set(previous_state);
                                                    admin_ir_request_id_timeout.set(0);
                                                    admin_ir_busy_timeout.set(false);
                                                }
                                            });

                                            let mut admin_ir_request_id_ack = admin_ir_request_id;
                                            let mut admin_ir_enabled_ack = admin_ir_enabled;
                                            let mut admin_ir_busy_ack = admin_ir_busy;
                                            spawn(async move {
                                                if admin_ir_request_id_ack() != request_id {
                                                    return;
                                                }

                                                match admin_toggle_ir_led_server(token, next_state).await {
                                                    Ok(state) => {
                                                        if admin_ir_request_id_ack() == request_id {
                                                            admin_ir_enabled_ack.set(state);
                                                            admin_ir_request_id_ack.set(0);
                                                            admin_ir_busy_ack.set(false);
                                                            device_refresh += 1;
                                                        }
                                                    }
                                                    Err(err) => {
                                                        if admin_ir_request_id_ack() == request_id {
                                                            admin_ir_enabled_ack.set(previous_state);
                                                            admin_ir_request_id_ack.set(0);
                                                            admin_ir_busy_ack.set(false);
                                                            let text = err.to_string();
                                                            if text.contains("Unauthorized") {
                                                                handle_unauthorized();
                                                            } else {
                                                                status.set(Some(format!("IR LED toggle failed: {}", text)));
                                                            }
                                                        }
                                                    }
                                                }
                                            });
                                        },
                                        span {
                                            class: format!(
                                                "inline-block h-4 w-4 transform rounded-full bg-white transition-transform {}",
                                                if admin_ir_enabled() { "translate-x-7" } else { "translate-x-1" }
                                            )
                                        }
                                    }
                                }

                                button {
                                    r#type: "button",
                                    class: format!(
                                        "rounded-md px-4 py-2 font-medium {}",
                                        if admin_save_busy() {
                                            "bg-slate-500 text-white"
                                        } else {
                                            "bg-emerald-500 hover:bg-emerald-600 text-white"
                                        }
                                    ),
                                    disabled: admin_save_busy(),
                                    onclick: move |_| {
                                        if admin_save_busy() {
                                            return;
                                        }
                                        let Some(token) = admin_token() else {
                                            handle_unauthorized();
                                            return;
                                        };

                                        admin_save_busy.set(true);
                                        status.set(None);
                                        spawn(async move {
                                            match admin_save_image_server(token).await {
                                                Ok(_msg) => {}
                                                Err(err) => {
                                                    let text = err.to_string();
                                                    if text.contains("Unauthorized") {
                                                        handle_unauthorized();
                                                    } else {
                                                        status.set(Some(format!("Save image failed: {}", text)));
                                                    }
                                                }
                                            }
                                            admin_save_busy.set(false);
                                        });
                                    },
                                    if admin_save_busy() { "Saving image..." } else { "Save Image" }
                                }
                            }
                        }
                    }

                    div { class: "grid grid-cols-1 xl:grid-cols-1 gap-6",
                        div {
                            class: "rounded-xl border border-slate-700 bg-slate-800 p-6 space-y-4",
                            h2 { class: "text-xl font-medium", "Add Picture to Gallery" }
                            p { class: "text-slate-300 text-sm", "Allowed formats: jpg, jpeg, png, gif, webp." }
                            input {
                                r#type: "file",
                                accept: ".jpg,.jpeg,.png,.gif,.webp,image/jpeg,image/png,image/gif,image/webp",
                                class: "block w-full text-sm text-slate-200 file:mr-4 file:rounded-md file:border-0 file:bg-slate-600 file:px-3 file:py-2 file:text-white hover:file:bg-slate-500",
                                onchange: move |evt| {
                                    let files = evt.files();
                                    let Some(file) = files.first().cloned() else { return; };
                                    spawn(async move {
                                        match file.read_bytes().await {
                                            Ok(bytes) => {
                                                upload_filename.set(Some(file.name()));
                                                upload_bytes.set(Some(bytes.to_vec()));
                                            }
                                            Err(_) => {
                                                status.set(Some("Failed to read selected file.".to_string()));
                                            }
                                        }
                                    });
                                },
                            }
                            if let Some(name) = upload_filename() {
                                p { class: "text-sm text-slate-300", "Selected: {name}" }
                            }
                            button {
                                r#type: "button",
                                class: format!(
                                    "rounded-md px-4 py-2 font-medium {}",
                                    if upload_busy() {
                                        "bg-slate-500 text-white"
                                    } else {
                                        "bg-emerald-500 hover:bg-emerald-600 text-white"
                                    }
                                ),
                                disabled: upload_busy(),
                                onclick: move |_| {
                                    if upload_busy() {
                                        return;
                                    }
                                    let Some(token) = admin_token() else {
                                        handle_unauthorized();
                                        return;
                                    };
                                    let Some(filename) = upload_filename() else {
                                        status.set(Some("Select a file first.".to_string()));
                                        return;
                                    };
                                    let Some(bytes) = upload_bytes() else {
                                        status.set(Some("Failed to read file bytes.".to_string()));
                                        return;
                                    };

                                    upload_busy.set(true);
                                    status.set(None);
                                    spawn(async move {
                                        match admin_upload_gallery_image_server(token, filename, bytes).await {
                                            Ok(()) => {
                                                upload_filename.set(None);
                                                upload_bytes.set(None);
                                                gallery_refresh += 1;
                                                status.set(Some("Image uploaded.".to_string()));
                                            }
                                            Err(err) => {
                                                let text = err.to_string();
                                                if text.contains("Unauthorized") {
                                                    handle_unauthorized();
                                                } else {
                                                    status.set(Some(format!("Upload failed: {}", text)));
                                                }
                                            }
                                        }
                                        upload_busy.set(false);
                                    });
                                },
                                if upload_busy() { "Uploading..." } else { "Upload Image" }
                            }
                        }
                    }

                    div {
                        class: "rounded-xl border border-slate-700 bg-slate-800 p-6 space-y-4",
                        h2 { class: "text-xl font-medium", "Gallery Manager" }
                        p { class: "text-slate-300 text-sm", "Delete existing pictures from the gallery." }

                        if let Some(Ok(Some(images))) = gallery_resource.read().as_ref() {
                            if images.is_empty() {
                                p { class: "text-slate-300", "Gallery is empty." }
                            } else {
                                div { class: "grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4",
                                    for img in images.iter() {
                                        div {
                                            key: "{img.filename}",
                                            class: "rounded-lg overflow-hidden border border-slate-700 bg-slate-900",
                                            img {
                                                src: "{img.thumbnail_url}",
                                                alt: "{img.filename}",
                                                class: "w-full h-36 object-cover"
                                            }
                                            div { class: "p-2 space-y-2",
                                                p { class: "text-xs text-slate-200 truncate", "{img.filename}" }
                                                button {
                                                    r#type: "button",
                                                    class: "w-full rounded-md px-2 py-1 text-sm bg-red-600 hover:bg-red-700 text-white",
                                                    onclick: {
                                                        let filename = img.filename.clone();
                                                        move |_| {
                                                            let Some(token) = admin_token() else {
                                                                handle_unauthorized();
                                                                return;
                                                            };
                                                            status.set(None);
                                                            let filename_for_request = filename.clone();
                                                            spawn(async move {
                                                                match admin_delete_gallery_image_server(token, filename_for_request).await {
                                                                    Ok(()) => {
                                                                        gallery_refresh += 1;
                                                                        status.set(Some("Image removed.".to_string()));
                                                                    }
                                                                    Err(err) => {
                                                                        let text = err.to_string();
                                                                        if text.contains("Unauthorized") {
                                                                            handle_unauthorized();
                                                                        } else {
                                                                            status.set(Some(format!("Delete failed: {}", text)));
                                                                        }
                                                                    }
                                                                }
                                                            });
                                                        }
                                                    },
                                                    "Delete"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else if let Some(Err(err)) = gallery_resource.read().as_ref() {
                            p { class: "text-sm text-red-300 break-all", "Failed to load gallery: {err}" }
                        } else if let Some(Ok(None)) = gallery_resource.read().as_ref() {
                            p { class: "text-slate-300", "Please sign in to load gallery items." }
                        } else {
                            p { class: "text-slate-300", "Loading gallery..." }
                        }
                    }
                }
            }
        }
    }
}
