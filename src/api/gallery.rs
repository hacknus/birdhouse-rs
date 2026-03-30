use axum::{
    extract::{Multipart, Path as AxumPath},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
};
use image::{codecs::jpeg::JpegEncoder, ImageReader};
use std::path::{Path, PathBuf};
use tokio::fs;

const GALLERY_DIR: &str = "./gallery";
const THUMB_CACHE_DIR: &str = "./gallery-thumbs";
const THUMB_MAX_SIZE: u32 = 640;
const THUMB_QUALITY: u8 = 75;
const MAX_UPLOAD_BYTES: usize = 20 * 1024 * 1024;

enum UploadAssetKind {
    StandardImage,
    LivePhotoStill { bundle_id: String },
    LivePhotoMotion { bundle_id: String },
}

fn sanitize_gallery_filename(filename: &str) -> Option<String> {
    let safe_filename: String = filename
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
        .collect();

    if safe_filename.is_empty() || safe_filename != filename {
        return None;
    }

    Some(safe_filename)
}

fn is_supported_image(path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(
        extension.as_str(),
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "heic"
    )
}

fn is_thumbnail_source(path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(extension.as_str(), "jpg" | "jpeg" | "png" | "gif" | "webp")
}

fn is_supported_motion(path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(extension.as_str(), "mov" | "mp4")
}

fn content_type_for_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "heic" => "image/heic",
        "mov" => "video/quicktime",
        "mp4" => "video/mp4",
        _ => "application/octet-stream",
    }
}

fn sanitize_bundle_id(bundle_id: &str) -> Option<String> {
    let safe_bundle_id: String = bundle_id
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
        .collect();

    if safe_bundle_id.is_empty() || safe_bundle_id != bundle_id {
        return None;
    }

    Some(safe_bundle_id)
}

fn parse_upload_asset_kind(
    filename: &str,
    asset_kind: Option<&str>,
    bundle_id: Option<&str>,
) -> Result<UploadAssetKind, &'static str> {
    match asset_kind {
        None => Ok(UploadAssetKind::StandardImage),
        Some("live_photo_still") => {
            let bundle_id = bundle_id
                .and_then(sanitize_bundle_id)
                .ok_or("invalid bundle_id")?;
            if !is_supported_image(Path::new(filename)) {
                return Err("invalid live photo still type");
            }
            Ok(UploadAssetKind::LivePhotoStill { bundle_id })
        }
        Some("live_photo_motion") => {
            let bundle_id = bundle_id
                .and_then(sanitize_bundle_id)
                .ok_or("invalid bundle_id")?;
            if !is_supported_motion(Path::new(filename)) {
                return Err("invalid live photo motion type");
            }
            Ok(UploadAssetKind::LivePhotoMotion { bundle_id })
        }
        Some(_) => Err("invalid asset_kind"),
    }
}

fn extension_for_filename(filename: &str) -> Option<String> {
    Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
}

async fn should_regenerate_thumbnail(source_path: &Path, thumb_path: &Path) -> bool {
    let source_meta = match fs::metadata(source_path).await {
        Ok(meta) => meta,
        Err(_) => return true,
    };

    let thumb_meta = match fs::metadata(thumb_path).await {
        Ok(meta) => meta,
        Err(_) => return true,
    };

    match (source_meta.modified(), thumb_meta.modified()) {
        (Ok(source_time), Ok(thumb_time)) => source_time > thumb_time,
        _ => false,
    }
}

fn thumbnail_filename(filename: &str) -> String {
    format!("{}.jpg", filename)
}

pub async fn serve_gallery_thumbnail(AxumPath(filename): AxumPath<String>) -> impl IntoResponse {
    let Some(safe_filename) = sanitize_gallery_filename(&filename) else {
        return (StatusCode::BAD_REQUEST, "invalid filename").into_response();
    };

    let source_path = PathBuf::from(GALLERY_DIR).join(&safe_filename);
    let source_meta = match fs::metadata(&source_path).await {
        Ok(meta) => meta,
        Err(_) => return (StatusCode::NOT_FOUND, "image not found").into_response(),
    };
    if source_meta.len() == 0 {
        return (StatusCode::NOT_FOUND, "image not found").into_response();
    }

    if !is_supported_image(&source_path) {
        return (StatusCode::BAD_REQUEST, "invalid file type").into_response();
    }

    if !is_thumbnail_source(&source_path) {
        let original = match fs::read(&source_path).await {
            Ok(bytes) => bytes,
            Err(_) => return (StatusCode::NOT_FOUND, "image not found").into_response(),
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            content_type_for_path(&source_path).parse().unwrap(),
        );
        headers.insert(
            header::CACHE_CONTROL,
            "public, max-age=604800, immutable".parse().unwrap(),
        );
        return (StatusCode::OK, headers, original).into_response();
    }

    if let Err(e) = fs::create_dir_all(THUMB_CACHE_DIR).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("mkdir failed: {}", e),
        )
            .into_response();
    }

    let thumb_path = PathBuf::from(THUMB_CACHE_DIR).join(thumbnail_filename(&safe_filename));

    if !should_regenerate_thumbnail(&source_path, &thumb_path).await {
        if let Ok(cached) = fs::read(&thumb_path).await {
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "image/jpeg".parse().unwrap());
            headers.insert(
                header::CACHE_CONTROL,
                "public, max-age=604800, immutable".parse().unwrap(),
            );
            return (StatusCode::OK, headers, cached).into_response();
        }
    }

    let source_path_for_task = source_path.clone();
    let thumb_path_for_task = thumb_path.clone();
    let generated = match tokio::task::spawn_blocking(move || -> Result<Vec<u8>, String> {
        let image = ImageReader::open(&source_path_for_task)
            .map_err(|e| format!("open failed: {}", e))?
            .with_guessed_format()
            .map_err(|e| format!("format guess failed: {}", e))?
            .decode()
            .map_err(|e| format!("decode failed: {}", e))?;

        let thumb = image.thumbnail(THUMB_MAX_SIZE, THUMB_MAX_SIZE);
        let mut bytes = Vec::new();
        let mut encoder = JpegEncoder::new_with_quality(&mut bytes, THUMB_QUALITY);
        encoder
            .encode_image(&thumb)
            .map_err(|e| format!("encode failed: {}", e))?;

        std::fs::write(&thumb_path_for_task, &bytes)
            .map_err(|e| format!("cache write failed: {}", e))?;
        Ok(bytes)
    })
    .await
    {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(msg)) => {
            if msg.contains("decode failed") || msg.contains("format guess failed") {
                return (StatusCode::UNPROCESSABLE_ENTITY, "invalid image").into_response();
            }
            return (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("thumbnail task failed: {}", e),
            )
                .into_response()
        }
    };

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "image/jpeg".parse().unwrap());
    headers.insert(
        header::CACHE_CONTROL,
        "public, max-age=604800, immutable".parse().unwrap(),
    );
    (StatusCode::OK, headers, generated).into_response()
}

pub async fn upload_image_multipart(mut multipart: Multipart) -> impl IntoResponse {
    let mut file_bytes: Option<bytes::Bytes> = None;
    let mut filename: Option<String> = None;
    let mut auth_token: Option<String> = None;
    let mut bundle_id: Option<String> = None;
    let mut asset_kind: Option<String> = None;

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
            Some(n) if n == "bundle_id" => {
                if let Ok(t) = field.text().await {
                    bundle_id = Some(t);
                }
            }
            Some(n) if n == "asset_kind" => {
                if let Ok(t) = field.text().await {
                    asset_kind = Some(t);
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
    if bytes.len() > MAX_UPLOAD_BYTES {
        return (StatusCode::PAYLOAD_TOO_LARGE, "file too large (max 20 MB)").into_response();
    }

    let safe_filename = match sanitize_gallery_filename(&filename) {
        Some(name) => name,
        None => return (StatusCode::BAD_REQUEST, "invalid filename").into_response(),
    };

    let asset_kind = match parse_upload_asset_kind(
        &safe_filename,
        asset_kind.as_deref(),
        bundle_id.as_deref(),
    ) {
        Ok(kind) => kind,
        Err(message) => return (StatusCode::BAD_REQUEST, message).into_response(),
    };

    if matches!(asset_kind, UploadAssetKind::StandardImage) && !is_supported_image(Path::new(&safe_filename)) {
        return (StatusCode::BAD_REQUEST, "invalid file type").into_response();
    }

    if let Err(e) = fs::create_dir_all(GALLERY_DIR).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("mkdir failed: {}", e),
        )
            .into_response();
    }

    let extension = match extension_for_filename(&safe_filename) {
        Some(ext) => ext,
        None => return (StatusCode::BAD_REQUEST, "missing filename extension").into_response(),
    };

    let stored_filename = match &asset_kind {
        UploadAssetKind::StandardImage => safe_filename.clone(),
        UploadAssetKind::LivePhotoStill { bundle_id } => format!("{}.{}", bundle_id, extension),
        UploadAssetKind::LivePhotoMotion { bundle_id } => format!("{}.{}", bundle_id, extension),
    };

    let path = format!("{}/{}", GALLERY_DIR, stored_filename);
    if let Err(e) = fs::write(&path, &bytes).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("write failed: {}", e),
        )
            .into_response();
    }

    if !matches!(asset_kind, UploadAssetKind::LivePhotoMotion { .. }) {
        // Drop any stale thumbnail cache so replaced files regenerate.
        let thumb_path = PathBuf::from(THUMB_CACHE_DIR).join(thumbnail_filename(&stored_filename));
        let _ = fs::remove_file(thumb_path).await;
    }

    let response_message = match asset_kind {
        UploadAssetKind::StandardImage => format!("uploaded {}", path),
        UploadAssetKind::LivePhotoStill { bundle_id } => {
            format!("uploaded live photo still {} ({})", path, bundle_id)
        }
        UploadAssetKind::LivePhotoMotion { bundle_id } => {
            format!("uploaded live photo motion {} ({})", path, bundle_id)
        }
    };

    (StatusCode::OK, response_message).into_response()
}
