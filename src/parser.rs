//! Input parser - fetch HTML from URL or file

use anyhow::Result;
use std::sync::OnceLock;
use url::Url;

// Maximum sizes for security
const MAX_HTML_SIZE: usize = 100_000_000; // 100MB
const MAX_SPEC_SIZE: usize = 1_048_576; // 1MB
const HTTP_TIMEOUT_SECONDS: u64 = 30;

// Singleton HTTP client for connection pooling
static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn get_http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECONDS))
            .connect_timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECONDS))
            .pool_max_idle_per_host(10)
            .build()
            .unwrap_or_else(|e| panic!("Failed to build HTTP client: {}", e))
    })
}

/// Fetch HTML from a URL or read from a file path
pub async fn fetch_html(input: &str) -> Result<String> {
    match url::Url::parse(input) {
        Ok(url) => fetch_from_url(&url).await,
        _ => read_from_file(input),
    }
}

/// Fetch HTML from a URL
async fn fetch_from_url(url: &Url) -> Result<String> {
    // Validate URL scheme before fetching
    if !matches!(url.scheme(), "http" | "https") {
        return Err(anyhow::anyhow!(
            "Unsupported URL scheme '{}': only http and https are allowed",
            url.scheme()
        ));
    }

    let response = get_http_client().get(url.clone()).send().await?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "HTTP error {}: failed to fetch {}",
            response.status(),
            url
        ));
    }

    let html = response.text().await?;
    if html.len() > MAX_HTML_SIZE {
        return Err(anyhow::anyhow!(
            "HTML input exceeds maximum size of {} bytes",
            MAX_HTML_SIZE
        ));
    }

    Ok(html)
}

/// Read HTML from a file path
fn read_from_file(path: &str) -> Result<String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read file '{}': {}", path, e))?;

    if content.len() > MAX_HTML_SIZE {
        return Err(anyhow::anyhow!(
            "HTML input exceeds maximum size of {} bytes",
            MAX_HTML_SIZE
        ));
    }

    Ok(content)
}

/// Load spec from a JSON file
pub fn load_spec(path: &str) -> Result<serde_json::Value> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read spec file '{}': {}", path, e))?;

    if content.len() > MAX_SPEC_SIZE {
        return Err(anyhow::anyhow!(
            "Spec file exceeds maximum size of {} bytes",
            MAX_SPEC_SIZE
        ));
    }

    let value: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse spec JSON: {}", e))?;

    Ok(value)
}
