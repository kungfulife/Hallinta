use crate::models::Catalog;
use sha2::{Digest, Sha256};

pub async fn fetch_catalog(catalog_url: &str) -> Result<Catalog, String> {
    if catalog_url.is_empty() {
        return Err("Preset catalog URL is not configured".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client.get(catalog_url).send().await.map_err(|e| {
        if e.is_timeout() {
            "Request timed out. Please check your internet connection and try again.".to_string()
        } else if e.is_connect() {
            "Could not connect to the catalog server. Please check your internet connection."
                .to_string()
        } else {
            format!("Failed to fetch catalog: {}", e)
        }
    })?;

    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(
            "Catalog not found (404). The configured catalog source may be unavailable."
                .to_string(),
        );
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err("Rate limited by the server. Please try again later.".to_string());
    }
    if !status.is_success() {
        return Err(format!(
            "Server returned error: {} {}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("Unknown")
        ));
    }

    let catalog: Catalog = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse catalog JSON: {}", e))?;

    if catalog.catalog_version.is_empty() {
        return Err("Invalid catalog: missing catalog_version".to_string());
    }

    Ok(catalog)
}

pub async fn download_preset_file(download_url: &str) -> Result<String, String> {
    if download_url.is_empty() {
        return Err("No download URL provided".to_string());
    }

    // Auto-convert Google Drive share links to direct download URLs
    let resolved_url = parse_gdrive_share_link(download_url)
        .unwrap_or_else(|| download_url.to_string());

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client.get(&resolved_url).send().await.map_err(|e| {
        if e.is_timeout() {
            "Download timed out. Please try again.".to_string()
        } else if e.is_connect() {
            "Could not connect to download server.".to_string()
        } else {
            format!("Failed to download preset: {}", e)
        }
    })?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "Download failed: {} {}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("Unknown")
        ));
    }

    response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))
}

pub fn parse_gdrive_share_link(url: &str) -> Option<String> {
    let file_id = extract_gdrive_file_id(url)?;
    Some(format!(
        "https://drive.google.com/uc?export=download&id={}",
        file_id
    ))
}

fn extract_gdrive_file_id(url: &str) -> Option<String> {
    if let Some(rest) = url.strip_prefix("https://drive.google.com/file/d/") {
        if let Some(id) = rest.split('/').next() {
            if !id.is_empty() {
                return Some(id.to_string());
            }
        }
    }

    if url.contains("drive.google.com") || url.contains("drive.usercontent.google.com") {
        if let Some(pos) = url.find("id=") {
            let after_id = &url[pos + 3..];
            let id: String = after_id
                .chars()
                .take_while(|c| *c != '&' && *c != '#')
                .collect();
            if !id.is_empty() {
                return Some(id);
            }
        }
    }

    None
}

pub fn verify_checksum(content: &str, expected_checksum: &str) -> bool {
    let expected_hex = expected_checksum
        .strip_prefix("sha256:")
        .unwrap_or(expected_checksum);

    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    let computed_hex = format!("{:x}", result);

    computed_hex == expected_hex
}

/// BUG-7 FIX: Compute checksum over canonical serialization (sorted keys via BTreeMap).
pub fn compute_checksum(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    format!("sha256:{:x}", result)
}
