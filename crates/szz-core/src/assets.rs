use crate::error::{Result, SzzError};
use crate::loaders::common::fetch_mojang_version_manifest;
use crate::paths::SzzPaths;
use crate::progress::{LaunchStep, ProgressReporter};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

const MOJANG_RESOURCES_BASE_URL: &str = "https://resources.download.minecraft.net";

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndexFile {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetSyncResult {
    pub asset_index_id: String,
    pub total_objects: usize,
    pub downloaded_objects: usize,
}

/// Validates that a hash is a 40-character hexadecimal string to prevent path traversal attacks.
pub fn validate_hash(hash: &str) -> Result<()> {
    if crate::manifest::HashAlgorithm::Sha1.is_valid_hex(hash) {
        Ok(())
    } else {
        Err(SzzError::Security(format!(
            "Invalid asset hash (must be 40-char hex): '{}'",
            hash
        )))
    }
}

/// Validates that an asset index ID contains no path traversal sequences or directory separators.
pub fn validate_asset_index_id(id: &str) -> Result<()> {
    if id.is_empty() || id.contains("..") || id.contains('/') || id.contains('\\') || id.contains(':') {
        Err(SzzError::Security(format!(
            "Invalid asset index ID (path traversal detected): '{}'",
            id
        )))
    } else {
        Ok(())
    }
}

async fn download_single_asset_with_retry(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    expected_hash: &str,
    expected_size: u64,
) -> Result<()> {
    let mut attempts = 0;
    let max_attempts = 3;
    let backoffs = [100, 300];

    loop {
        attempts += 1;
        match download_single_asset(client, url, dest, expected_hash, expected_size).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                if attempts >= max_attempts {
                    return Err(e);
                }
                let delay = backoffs[attempts - 1];
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            }
        }
    }
}

fn write_atomic(dest: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|source| SzzError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let filename = dest
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let nonce = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp_file = dest.with_file_name(format!("{}.tmp-{}-{}", filename, std::process::id(), nonce));

    std::fs::write(&temp_file, bytes).map_err(|source| SzzError::Io {
        path: temp_file.clone(),
        source,
    })?;

    let _ = std::fs::remove_file(dest);

    std::fs::rename(&temp_file, dest).map_err(|source| SzzError::Io {
        path: dest.to_path_buf(),
        source,
    })?;

    Ok(())
}

async fn download_single_asset(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    expected_hash: &str,
    expected_size: u64,
) -> Result<()> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|source| SzzError::Network {
            url: url.to_string(),
            source,
        })?;

    if !response.status().is_success() {
        return Err(SzzError::Assets(format!(
            "HTTP {} downloading asset from {}",
            response.status(),
            url
        )));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|source| SzzError::Network {
            url: url.to_string(),
            source,
        })?;

    if bytes.len() as u64 != expected_size {
        return Err(SzzError::Assets(format!(
            "Size mismatch for asset {}: expected {} bytes, got {}",
            expected_hash,
            expected_size,
            bytes.len()
        )));
    }

    let actual_hash = hex::encode(Sha1::digest(&bytes));
    if !actual_hash.eq_ignore_ascii_case(expected_hash) {
        return Err(SzzError::Assets(format!(
            "SHA1 mismatch for asset {}: expected {}, got {}",
            dest.display(),
            expected_hash,
            actual_hash
        )));
    }

    write_atomic(dest, &bytes)
}

async fn fetch_or_load_asset_index(
    paths: &SzzPaths,
    index_id: &str,
    index_url: &str,
    index_sha1: &str,
    client: &reqwest::Client,
) -> Result<AssetIndexFile> {
    validate_asset_index_id(index_id)?;
    validate_hash(index_sha1)?;

    let index_file = paths.asset_index_path(index_id);

    if let Ok(bytes) = std::fs::read(&index_file) {
        let actual_hash = hex::encode(Sha1::digest(&bytes));
        if actual_hash.eq_ignore_ascii_case(index_sha1) {
            if let Ok(parsed) = serde_json::from_slice::<AssetIndexFile>(&bytes) {
                return Ok(parsed);
            }
        }
    }

    let response = client
        .get(index_url)
        .send()
        .await
        .map_err(|source| SzzError::Network {
            url: index_url.to_string(),
            source,
        })?;

    if !response.status().is_success() {
        return Err(SzzError::Assets(format!(
            "HTTP {} downloading asset index from {}",
            response.status(),
            index_url
        )));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|source| SzzError::Network {
            url: index_url.to_string(),
            source,
        })?;

    let actual_hash = hex::encode(Sha1::digest(&bytes));
    if !actual_hash.eq_ignore_ascii_case(index_sha1) {
        return Err(SzzError::Assets(format!(
            "SHA1 mismatch for asset index {}: expected {}, got {}",
            index_id, index_sha1, actual_hash
        )));
    }

    // Parse and validate JSON structure before committing to disk cache
    let parsed: AssetIndexFile = serde_json::from_slice(&bytes)?;

    write_atomic(&index_file, &bytes)?;

    Ok(parsed)
}

/// Synchronizes official Mojang assets (sounds, languages, fonts) into CAS storage.
///
/// If `dry_run` is true, skips all network requests and returns immediately with
/// resolved or default metadata.
pub async fn sync_mojang_assets(
    paths: &SzzPaths,
    mc_version: &str,
    reporter: &dyn ProgressReporter,
    dry_run: bool,
) -> Result<AssetSyncResult> {
    if dry_run {
        let version_json_file = paths
            .version_dir(mc_version)
            .join(format!("{}.json", mc_version));

        let asset_index_id = std::fs::read(&version_json_file)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<crate::loaders::MojangVersionManifest>(&bytes).ok())
            .and_then(|manifest| manifest.asset_index)
            .map(|a| a.id)
            .unwrap_or_else(|| mc_version.to_string());

        return Ok(AssetSyncResult {
            asset_index_id,
            total_objects: 0,
            downloaded_objects: 0,
        });
    }

    let mojang_manifest = fetch_mojang_version_manifest(paths, mc_version).await?;

    let asset_index_meta = match mojang_manifest.asset_index {
        Some(meta) => meta,
        None => {
            return Ok(AssetSyncResult {
                asset_index_id: mc_version.to_string(),
                total_objects: 0,
                downloaded_objects: 0,
            });
        }
    };

    let client = reqwest::Client::builder()
        .user_agent("szz-launcher/0.1.0")
        .pool_max_idle_per_host(32)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|source| SzzError::Network {
            url: asset_index_meta.url.clone(),
            source,
        })?;

    let index_file = fetch_or_load_asset_index(
        paths,
        &asset_index_meta.id,
        &asset_index_meta.url,
        &asset_index_meta.sha1,
        &client,
    )
    .await?;

    let total_objects = index_file.objects.len();
    let objects_dir = paths.assets_objects_dir();

    // Fast local cache check: filter missing objects using metadata size check
    let mut seen_hashes = HashSet::with_capacity(total_objects);
    let mut missing_objects = Vec::new();
    let mut obj_dest = objects_dir.clone();

    for obj in index_file.objects.values() {
        validate_hash(&obj.hash)?;
        let hash_lower = obj.hash.to_ascii_lowercase();
        if seen_hashes.insert(hash_lower.clone()) {
            obj_dest.push(&hash_lower[0..2]);
            obj_dest.push(&hash_lower);
            let is_cached = std::fs::metadata(&obj_dest)
                .map(|meta| meta.is_file() && meta.len() == obj.size)
                .unwrap_or(false);
            obj_dest.pop();
            obj_dest.pop();

            if !is_cached {
                missing_objects.push((hash_lower, obj.size));
            }
        }
    }

    let missing_count = missing_objects.len();
    if missing_count == 0 {
        return Ok(AssetSyncResult {
            asset_index_id: asset_index_meta.id,
            total_objects,
            downloaded_objects: 0,
        });
    }

    // Pre-create prefix bucket directories sequentially to eliminate lock contention among concurrent workers
    let mut created_dirs = HashSet::new();
    for (hash, _) in &missing_objects {
        let prefix = &hash[0..2];
        if created_dirs.insert(prefix) {
            let bucket_dir = objects_dir.join(prefix);
            std::fs::create_dir_all(&bucket_dir).map_err(|source| SzzError::Io {
                path: bucket_dir,
                source,
            })?;
        }
    }

    let semaphore = Arc::new(Semaphore::new(32));
    let mut join_set = JoinSet::new();

    for (hash_lower, size) in missing_objects {
        let sem = semaphore.clone();
        let client_ref = client.clone();
        let dest = objects_dir.join(&hash_lower[0..2]).join(&hash_lower);
        let url = format!(
            "{}/{}/{}",
            MOJANG_RESOURCES_BASE_URL,
            &hash_lower[0..2],
            hash_lower
        );

        join_set.spawn(async move {
            let _permit = sem
                .acquire()
                .await
                .map_err(|e| SzzError::Assets(format!("Semaphore acquire failed: {}", e)))?;

            download_single_asset_with_retry(&client_ref, &url, &dest, &hash_lower, size).await
        });
    }

    let mut completed = 0;
    let mut last_reported = 0;
    while let Some(res) = join_set.join_next().await {
        res.map_err(|e| SzzError::Assets(format!("Asset task join error: {}", e)))??;
        completed += 1;
        if completed - last_reported >= 100 || completed == missing_count {
            reporter.report(
                LaunchStep::CheckingJavaRuntime,
                Some(&format!(
                    "Downloading Minecraft assets: {}/{} objects...",
                    completed, missing_count
                )),
            );
            last_reported = completed;
        }
    }

    Ok(AssetSyncResult {
        asset_index_id: asset_index_meta.id,
        total_objects,
        downloaded_objects: missing_count,
    })
}
