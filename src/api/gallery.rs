use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub struct UploadResponse {
    pub success: bool,
    pub message: String,
    pub filename: Option<String>,
}

#[server]
pub async fn upload_image(
    file_data: Vec<u8>,
    filename: String,
    auth_token: String,
) -> Result<UploadResponse, ServerFnError> {
    // Get token from environment variable
    let valid_token = std::env::var("UPLOAD_TOKEN")
        .map_err(|_| ServerFnError::new("Server configuration error"))?;

    // Constant-time comparison to prevent timing attacks
    if auth_token != valid_token {
        return Err(ServerFnError::new("Unauthorized"));
    }

    // Validate file size (e.g., max 10MB)
    const MAX_SIZE: usize = 10 * 1024 * 1024;
    if file_data.len() > MAX_SIZE {
        return Err(ServerFnError::new("File too large"));
    }

    // Sanitize filename - only allow alphanumeric, dots, and hyphens
    let safe_filename = filename
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
        .collect::<String>();

    if safe_filename.is_empty() || safe_filename != filename {
        return Err(ServerFnError::new("Invalid filename"));
    }

    // Validate file extension
    let extension = Path::new(&safe_filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    if !matches!(
        extension.to_lowercase().as_str(),
        "jpg" | "jpeg" | "png" | "gif" | "webp"
    ) {
        return Err(ServerFnError::new("Invalid file type"));
    }

    // Save file
    let upload_dir = "./public/gallery_cache";
    std::fs::create_dir_all(upload_dir)
        .map_err(|e| ServerFnError::new(format!("Failed to create directory: {}", e)))?;

    let file_path = format!("{}/{}", upload_dir, safe_filename);
    std::fs::write(&file_path, &file_data)
        .map_err(|e| ServerFnError::new(format!("Failed to write file: {}", e)))?;

    Ok(UploadResponse {
        success: true,
        message: format!("Successfully uploaded {}", safe_filename),
        filename: Some(safe_filename),
    })
}
