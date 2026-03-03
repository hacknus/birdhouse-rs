use base64::{engine::general_purpose::URL_SAFE, Engine as _};

pub fn normalize_email(raw: &str) -> Option<String> {
    let email = raw.trim().to_lowercase();
    if email.is_empty() || email.contains(' ') {
        return None;
    }

    let (local, domain) = email.split_once('@')?;
    if local.is_empty() || domain.is_empty() || !domain.contains('.') {
        return None;
    }

    Some(email)
}

pub fn encode_email(email: &str, key: &str) -> Result<String, String> {
    if key.is_empty() {
        return Err("ENCODING key is empty".to_string());
    }

    let encrypted = xor_encrypt_decrypt(email.as_bytes(), key.as_bytes());
    Ok(URL_SAFE.encode(encrypted))
}

pub fn decode_email(encoded_email: &str, key: &str) -> Result<String, String> {
    if key.is_empty() {
        return Err("ENCODING key is empty".to_string());
    }

    let decoded = URL_SAFE
        .decode(encoded_email)
        .map_err(|e| format!("Invalid unsubscribe token: {}", e))?;
    let decrypted = xor_encrypt_decrypt(&decoded, key.as_bytes());

    String::from_utf8(decrypted).map_err(|e| format!("Invalid decoded email: {}", e))
}

pub fn newsletter_base_url() -> String {
    std::env::var("NEWSLETTER_BASE_URL")
        .or_else(|_| std::env::var("PUBLIC_BASE_URL"))
        .unwrap_or_else(|_| "http://localhost:8080".to_string())
}

pub fn build_unsubscribe_link(email: &str, base_url: &str, key: &str) -> Result<String, String> {
    let encoded = encode_email(email, key)?;
    Ok(format!(
        "{}/unsubscribe/{}",
        base_url.trim_end_matches('/'),
        encoded
    ))
}

fn xor_encrypt_decrypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, b)| b ^ key[i % key.len()])
        .collect()
}
