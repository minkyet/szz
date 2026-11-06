use crate::error::{Result, SzzError};
use crate::manifest::ServerManifest;
use crate::paths::SzzPaths;
use reqwest::header::{HeaderValue, IF_MODIFIED_SINCE, IF_NONE_MATCH};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Timeout for catalog and manifest network requests.
pub const CATALOG_NETWORK_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteCatalog {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub servers: Vec<RemoteCatalogServerEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteCatalogServerEntry {
    pub id: String,
    pub name: String,
    pub tagline: String,
    pub manifest_url: String,
    pub latest_manifest_version: u32,
    #[serde(default)]
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogCacheMeta {
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub fetched_at: u64,
}

#[derive(Debug, Clone)]
pub struct CatalogFetchOutput {
    pub catalog: RemoteCatalog,
    pub is_offline: bool,
    pub from_cache: bool,
}

/// Validates that the URL scheme is strictly http or https to prevent SSRF and unsafe protocol execution.
pub fn validate_url_scheme(raw_url: &str) -> Result<Url> {
    let parsed = Url::parse(raw_url).map_err(|e| SzzError::Config(format!("Invalid URL '{}': {}", raw_url, e)))?;
    let scheme = parsed.scheme().to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err(SzzError::Config(format!(
            "Unsupported URL scheme '{}' in '{}'. Only http and https are allowed.",
            scheme, raw_url
        )));
    }
    Ok(parsed)
}

fn load_json_file<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Loads the cached `catalog.json` from `paths.catalog_cache_file()`, if present and valid.
pub fn load_cached_catalog(cache_file: &Path) -> Option<RemoteCatalog> {
    load_json_file(cache_file)
}

/// Loads the cache metadata (`catalog.meta.json`) from `paths.catalog_meta_file()`.
pub fn load_cache_meta(meta_file: &Path) -> Option<CatalogCacheMeta> {
    load_json_file(meta_file)
}

/// Atomically persists `catalog.json` and `catalog.meta.json` to disk.
pub fn save_cached_catalog(
    paths: &SzzPaths,
    raw_json: &str,
    etag: Option<String>,
    last_modified: Option<String>,
) -> Result<()> {
    let cache_dir = paths.cache_dir();
    std::fs::create_dir_all(&cache_dir).map_err(|source| SzzError::Io {
        path: cache_dir.clone(),
        source,
    })?;

    let nonce = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let temp_catalog = cache_dir.join(format!("catalog.tmp-{}-{}", pid, nonce));
    let temp_meta = cache_dir.join(format!("catalog.meta.tmp-{}-{}", pid, nonce));

    std::fs::write(&temp_catalog, raw_json.as_bytes()).map_err(|source| SzzError::Io {
        path: temp_catalog.clone(),
        source,
    })?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let meta = CatalogCacheMeta {
        etag,
        last_modified,
        fetched_at: now,
    };
    let meta_json = serde_json::to_string_pretty(&meta)?;
    std::fs::write(&temp_meta, meta_json.as_bytes()).map_err(|source| SzzError::Io {
        path: temp_meta.clone(),
        source,
    })?;

    let target_catalog = paths.catalog_cache_file();
    let target_meta = paths.catalog_meta_file();

    std::fs::rename(&temp_catalog, &target_catalog).map_err(|source| SzzError::Io {
        path: target_catalog,
        source,
    })?;
    std::fs::rename(&temp_meta, &target_meta).map_err(|source| SzzError::Io {
        path: target_meta,
        source,
    })?;

    Ok(())
}

/// Fetches the remote server catalog, using ETag and Last-Modified conditional requests.
///
/// Returns cached data on `304 Not Modified`, or gracefully falls back to local cache
/// with `is_offline = true` on network failure.
pub async fn fetch_remote_catalog(
    client: &reqwest::Client,
    paths: &SzzPaths,
    url: &str,
) -> Result<CatalogFetchOutput> {
    let parsed_url = validate_url_scheme(url)?;
    let cache_file = paths.catalog_cache_file();
    let meta_file = paths.catalog_meta_file();
    let cached_opt = load_cached_catalog(&cache_file);
    let meta = if cached_opt.is_some() {
        load_cache_meta(&meta_file)
    } else {
        // If cached catalog.json is missing or invalid, purge stale meta to avoid 304 desync
        let _ = std::fs::remove_file(&meta_file);
        None
    };

    let mut req = client.get(parsed_url.clone()).timeout(CATALOG_NETWORK_TIMEOUT);
    if let Some(ref m) = meta {
        if let Some(ref etag) = m.etag {
            if let Ok(val) = HeaderValue::from_str(etag) {
                req = req.header(IF_NONE_MATCH, val);
            }
        }
        if let Some(ref lm) = m.last_modified {
            if let Ok(val) = HeaderValue::from_str(lm) {
                req = req.header(IF_MODIFIED_SINCE, val);
            }
        }
    }

    match req.send().await {
        Ok(resp) => {
            let status = resp.status();
            if status == reqwest::StatusCode::NOT_MODIFIED {
                if let Some(cached) = cached_opt {
                    return Ok(CatalogFetchOutput {
                        catalog: cached,
                        is_offline: false,
                        from_cache: true,
                    });
                }
                // Corrupted cache on 304: clean meta to recover on next request
                let _ = std::fs::remove_file(&meta_file);
                return Err(SzzError::Runtime(
                    "Server responded 304 Not Modified, but local cached catalog is unreadable".to_string(),
                ));
            } else if status.is_success() {
                let etag = resp
                    .headers()
                    .get(reqwest::header::ETAG)
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());
                let last_modified = resp
                    .headers()
                    .get(reqwest::header::LAST_MODIFIED)
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());

                let text = resp.text().await.map_err(|e| SzzError::Network {
                    url: url.to_string(),
                    source: e,
                })?;

                let catalog: RemoteCatalog = serde_json::from_str(&text)?;
                let _ = save_cached_catalog(paths, &text, etag, last_modified);

                return Ok(CatalogFetchOutput {
                    catalog,
                    is_offline: false,
                    from_cache: false,
                });
            }

            // Non-success status code (4xx, 5xx): attempt fallback to cached catalog
            if let Some(cached) = cached_opt {
                return Ok(CatalogFetchOutput {
                    catalog: cached,
                    is_offline: true,
                    from_cache: true,
                });
            }

            Err(SzzError::Runtime(format!(
                "Failed to fetch catalog from '{}': HTTP status {}",
                url, status
            )))
        }
        Err(e) => {
            // Network failure or DNS error: fallback to local cache if present
            if let Some(cached) = cached_opt {
                return Ok(CatalogFetchOutput {
                    catalog: cached,
                    is_offline: true,
                    from_cache: true,
                });
            }

            Err(SzzError::Network {
                url: url.to_string(),
                source: e,
            })
        }
    }
}

/// Downloads and validates a remote server manifest from its `manifest_url`.
pub async fn fetch_server_manifest(client: &reqwest::Client, manifest_url: &str) -> Result<ServerManifest> {
    let parsed_url = validate_url_scheme(manifest_url)?;
    let resp = client
        .get(parsed_url)
        .timeout(CATALOG_NETWORK_TIMEOUT)
        .send()
        .await
        .map_err(|e| SzzError::Network {
            url: manifest_url.to_string(),
            source: e,
        })?;

    if !resp.status().is_success() {
        return Err(SzzError::Runtime(format!(
            "Failed to fetch manifest from '{}': HTTP status {}",
            manifest_url,
            resp.status()
        )));
    }

    let text = resp.text().await.map_err(|e| SzzError::Network {
        url: manifest_url.to_string(),
        source: e,
    })?;

    let manifest: ServerManifest = text.parse()?;
    Ok(manifest)
}

/// Convenience function that uses a default HTTP client to fetch the remote catalog.
pub async fn fetch_catalog_default(paths: &SzzPaths, url: &str) -> Result<CatalogFetchOutput> {
    let client = reqwest::Client::new();
    fetch_remote_catalog(&client, paths, url).await
}

/// Convenience function that uses a default HTTP client to fetch a server manifest.
pub async fn fetch_server_manifest_default(manifest_url: &str) -> Result<ServerManifest> {
    let client = reqwest::Client::new();
    fetch_server_manifest(&client, manifest_url).await
}
