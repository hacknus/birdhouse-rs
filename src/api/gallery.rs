use axum::{
    extract::Multipart,
    http::StatusCode,
    response::IntoResponse,
};
use std::path::Path;
use tokio::fs;
use base64::engine::general_purpose;
use base64::Engine;

pub async fn upload_image_multipart(mut multipart: Multipart) -> impl IntoResponse {
    let mut file_bytes: Option<bytes::Bytes> = None;
    let mut filename: Option<String> = None;
    let mut auth_token: Option<String> = None;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        match field.name().map(|s| s.to_string()) {
            Some(n) if n == "file" => {
                if let Ok(b) = field.bytes().await {
                    file_bytes = Some(b);
                }
            }
            Some(n) if n == "filename" => {
                if let Ok(t) = field.text().await {
                    filename = Some(t);
                }
            }
            Some(n) if n == "auth_token" => {
                if let Ok(t) = field.text().await {
                    auth_token = Some(t);
                }
            }
            _ => {}
        }
    }

    let auth_token = match auth_token {
        Some(t) => t,
        None => return (StatusCode::BAD_REQUEST, "missing auth_token").into_response(),
    };

    // Validate token as your server expects
    let valid_token = std::env::var("UPLOAD_TOKEN").unwrap_or_default();
    if auth_token != valid_token {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }

    let filename = match filename {
        Some(f) => f,
        None => return (StatusCode::BAD_REQUEST, "missing filename").into_response(),
    };

    let bytes = match file_bytes {
        Some(b) => b,
        None => return (StatusCode::BAD_REQUEST, "missing file").into_response(),
    };

    // Sanitize and validate filename (same rules as your existing code)
    let safe_filename: String = filename
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
        .collect();

    if safe_filename.is_empty() || safe_filename != filename {
        return (StatusCode::BAD_REQUEST, "invalid filename").into_response();
    }

    // Validate extension
    let extension = Path::new(&safe_filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !matches!(extension.as_str(), "jpg" | "jpeg" | "png" | "gif" | "webp") {
        return (StatusCode::BAD_REQUEST, "invalid file type").into_response();
    }

    let upload_dir = "./public/gallery";
    if let Err(e) = fs::create_dir_all(upload_dir).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("mkdir failed: {}", e)).into_response();
    }

    let path = format!("{}/{}", upload_dir, safe_filename);
    if let Err(e) = fs::write(&path, &bytes).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("write failed: {}", e)).into_response();
    }

    (StatusCode::OK, format!("uploaded {}", path)).into_response()
}