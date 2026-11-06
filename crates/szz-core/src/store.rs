use crate::error::{Result, SzzError};
use crate::manifest::{HashAlgorithm, ModEntry};
use crate::paths::SzzPaths;
use crate::progress::{LaunchStep, ProgressReporter};
use sha1::Sha1;
use sha2::{Digest, Sha512};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

pub const DEFAULT_MOD_CONCURRENCY: usize = 6;
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn hash_file<D: Digest + Default>(path: &Path) -> Result<String> {
    let mut file = File::open(path).map_err(|source| SzzError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    let mut hasher = D::new();
    let mut buffer = [0u8; 8192];
    loop {
        let n = file.read(&mut buffer).map_err(|source| SzzError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn compute_file_hash(path: &Path, algo: HashAlgorithm) -> Result<String> {
    match algo {
        HashAlgorithm::Sha1 => hash_file::<Sha1>(path),
        HashAlgorithm::Sha512 => hash_file::<Sha512>(path),
    }
}

pub fn verify_file_hash(path: &Path, expected_hash: &str, algo: HashAlgorithm) -> Result<bool> {
    let actual = compute_file_hash(path, algo)?;
    Ok(actual.eq_ignore_ascii_case(expected_hash))
}

pub fn are_same_file(path_a: &Path, path_b: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let (Ok(meta_a), Ok(meta_b)) = (std::fs::metadata(path_a), std::fs::metadata(path_b)) {
            return meta_a.dev() == meta_b.dev() && meta_a.ino() == meta_b.ino();
        }
    }
    #[cfg(windows)]
    {
        // On Windows, metadata equality check
        if let (Ok(meta_a), Ok(meta_b)) = (std::fs::metadata(path_a), std::fs::metadata(path_b)) {
            return meta_a.len() == meta_b.len() && meta_a.modified().ok() == meta_b.modified().ok();
        }
    }
    false
}

pub fn file_hardlink_count(path: &Path) -> Result<u64> {
    let meta = std::fs::metadata(path).map_err(|source| SzzError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(meta.nlink())
    }

    #[cfg(windows)]
    {
        // MetadataExt::number_of_links is nightly-only in Rust std.
        // On Windows, hardlinks frequently fall back to copies and active_hashes
        // provides primary CAS mod protection. Safe stable fallback returns 1.
        Ok(1)
    }

    #[cfg(not(any(unix, windows)))]
    {
        Ok(1)
    }
}

pub fn create_hardlink_or_copy(source: &Path, destination: &Path) -> Result<()> {
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(|err| SzzError::Io {
            path: parent.to_path_buf(),
            source: err,
        })?;
    }

    if destination.exists() {
        if are_same_file(source, destination) {
            return Ok(());
        }
        let _ = std::fs::remove_file(destination);
    }

    match std::fs::hard_link(source, destination) {
        Ok(_) => Ok(()),
        Err(err) => {
            // Cross-device link fallback or permission error fallback
            if err.raw_os_error() == Some(18) /* EXDEV */ || cfg!(windows) {
                std::fs::copy(source, destination).map_err(|copy_err| SzzError::Io {
                    path: destination.to_path_buf(),
                    source: copy_err,
                })?;
                Ok(())
            } else {
                Err(SzzError::Io {
                    path: destination.to_path_buf(),
                    source: err,
                })
            }
        }
    }
}

/// Scans the CAS store directory (~/.szz/store/) and removes:
/// 1. Any lingering .tmp files created by the current process (matching `.tmp-{pid}-`).
/// 2. Any stale .tmp files older than 10 minutes (600s) left behind by crashed processes.
///
/// This prevents race conditions where one process's cleaner removes another active process's temp files.
pub fn clean_stale_temp_files(paths: &SzzPaths) -> Result<usize> {
    let store_dir = paths.store_dir();
    let entries = match std::fs::read_dir(&store_dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(source) => return Err(SzzError::Io { path: store_dir, source }),
    };

    let my_pid_marker = format!(".tmp-{}-", std::process::id());
    let now = std::time::SystemTime::now();
    let mut removed_count = 0;

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if !name.contains(".tmp") {
            continue;
        }

        let path = entry.path();
        let meta = match entry.metadata() {
            Ok(m) if m.is_file() => m,
            _ => continue,
        };

        let is_stale = match meta.modified() {
            Ok(modified) => now.duration_since(modified).map(|d| d.as_secs() > 600).unwrap_or(true),
            Err(_) => true,
        };

        if (name.contains(&my_pid_marker) || is_stale) && std::fs::remove_file(&path).is_ok() {
            removed_count += 1;
        }
    }
    Ok(removed_count)
}

async fn download_single_mod_to_cas(
    client: &reqwest::Client,
    store_dir: &Path,
    url: &str,
    expected_hash: &str,
    algo: HashAlgorithm,
    name: &str,
) -> Result<PathBuf> {
    let store_file_name = format!("{}.jar", expected_hash.to_lowercase());
    let store_path = store_dir.join(&store_file_name);

    if store_path.exists() && verify_file_hash(&store_path, expected_hash, algo)? {
        return Ok(store_path);
    }

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|source| SzzError::Network {
            url: url.to_string(),
            source,
        })?;

    if !response.status().is_success() {
        return Err(SzzError::Store(format!(
            "Failed to download mod '{}' from {}: HTTP {}",
            name,
            url,
            response.status()
        )));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|source| SzzError::Network {
            url: url.to_string(),
            source,
        })?;

    if !algo.verify_bytes(&bytes, expected_hash) {
        return Err(SzzError::Store(format!(
            "Hash mismatch for mod '{}' downloaded from {}",
            name, url
        )));
    }

    let nonce = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp_name = format!(
        "{}.tmp-{}-{}",
        expected_hash.to_lowercase(),
        std::process::id(),
        nonce
    );
    let temp_path = store_dir.join(&temp_name);

    std::fs::write(&temp_path, &bytes).map_err(|source| SzzError::Io {
        path: temp_path.clone(),
        source,
    })?;

    if let Err(e) = std::fs::rename(&temp_path, &store_path) {
        let _ = std::fs::remove_file(&temp_path);
        if store_path.exists() && verify_file_hash(&store_path, expected_hash, algo).unwrap_or(false) {
            return Ok(store_path);
        }
        return Err(SzzError::Io {
            path: store_path,
            source: e,
        });
    }

    Ok(store_path)
}

pub async fn sync_mods_with_reporter(
    paths: &SzzPaths,
    instance_id: &str,
    mods: &[ModEntry],
    reporter: &dyn ProgressReporter,
) -> Result<Vec<PathBuf>> {
    paths.ensure_instance_dirs(instance_id)?;
    let store_dir = paths.store_dir();
    std::fs::create_dir_all(&store_dir).map_err(|source| SzzError::Io {
        path: store_dir.clone(),
        source,
    })?;

    // 1. Clean any stale .tmp files from prior interrupted runs
    let _ = clean_stale_temp_files(paths);

    let instance_mods_dir = paths.instance_mods_dir(instance_id);

    // 2. Pre-deduplicate downloads by primary hash and preserve order of instance links
    struct ModToLink {
        hash_lower: String,
        file_name: String,
    }

    let mut unique_downloads: HashMap<String, (&ModEntry, HashAlgorithm)> = HashMap::new();
    let mut mods_to_link: Vec<ModToLink> = Vec::with_capacity(mods.len());

    for m in mods {
        let (expected_hash, algo) = m.primary_hash()?;
        let hash_lower = expected_hash.to_lowercase();
        let sanitized_file_name = m.filename();

        mods_to_link.push(ModToLink {
            hash_lower: hash_lower.clone(),
            file_name: sanitized_file_name,
        });

        unique_downloads.entry(hash_lower).or_insert((m, algo));
    }

    // 3. Filter out mods already cached in CAS
    let mut missing_unique_mods: Vec<(String, &ModEntry, HashAlgorithm)> = Vec::new();
    for (hash_lower, (m, algo)) in unique_downloads {
        let store_file = store_dir.join(format!("{}.jar", hash_lower));
        let is_cached = store_file.is_file()
            && store_file
                .metadata()
                .map(|m| m.len() > 0)
                .unwrap_or(false);
        if !is_cached {
            missing_unique_mods.push((hash_lower, m, algo));
        }
    }

    let total_to_download = missing_unique_mods.len();

    // 4. Download missing mods concurrently with semaphore pool
    if total_to_download > 0 {
        let client = reqwest::Client::builder()
            .user_agent("szz-launcher/0.1.0")
            .pool_max_idle_per_host(32)
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|source| SzzError::Store(format!("Failed to build HTTP client: {}", source)))?;

        let semaphore = Arc::new(Semaphore::new(DEFAULT_MOD_CONCURRENCY));
        let mut join_set = JoinSet::new();

        for (hash_lower, m, algo) in missing_unique_mods {
            let sem = semaphore.clone();
            let client_ref = client.clone();
            let store_dir_ref = store_dir.clone();
            let url = m.file_url.clone();
            let name = m.name.clone();

            join_set.spawn(async move {
                let _permit = sem
                    .acquire()
                    .await
                    .map_err(|e| SzzError::Store(format!("Semaphore acquire failed: {}", e)))?;

                download_single_mod_to_cas(
                    &client_ref,
                    &store_dir_ref,
                    &url,
                    &hash_lower,
                    algo,
                    &name,
                )
                .await
            });
        }

        let mut completed_downloads = 0;
        while let Some(res) = join_set.join_next().await {
            match res.map_err(|e| SzzError::Store(format!("Mod download task join error: {}", e))).and_then(|r| r) {
                Ok(_store_path) => {
                    completed_downloads += 1;
                    reporter.report(
                        LaunchStep::SyncingMods,
                        Some(&format!(
                            "Downloading mods: {}/{}...",
                            completed_downloads, total_to_download
                        )),
                    );
                }
                Err(err) => {
                    join_set.abort_all();
                    while join_set.join_next().await.is_some() {}
                    let _ = clean_stale_temp_files(paths);
                    return Err(err);
                }
            }
        }
    }

    // 5. Hardlink or copy all mods into instance_mods_dir
    let mut linked_paths = Vec::with_capacity(mods_to_link.len());
    for item in mods_to_link {
        let store_file = store_dir.join(format!("{}.jar", item.hash_lower));
        let instance_mod_path = instance_mods_dir.join(&item.file_name);
        create_hardlink_or_copy(&store_file, &instance_mod_path)?;
        linked_paths.push(instance_mod_path);
    }

    reporter.report(
        LaunchStep::SyncingMods,
        Some(&format!("Linked {} mods to instance", linked_paths.len())),
    );

    Ok(linked_paths)
}

pub async fn ensure_mod(
    paths: &SzzPaths,
    instance_id: &str,
    mod_entry: &ModEntry,
) -> Result<PathBuf> {
    let linked = sync_mods(paths, instance_id, std::slice::from_ref(mod_entry)).await?;
    linked
        .into_iter()
        .next()
        .ok_or_else(|| SzzError::Store("Failed to ensure mod".to_string()))
}

pub async fn sync_mods(
    paths: &SzzPaths,
    instance_id: &str,
    mods: &[ModEntry],
) -> Result<Vec<PathBuf>> {
    sync_mods_with_reporter(
        paths,
        instance_id,
        mods,
        &crate::progress::NoopProgressReporter,
    )
    .await
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PruneSummary {
    pub reclaimed_bytes: u64,
    pub pruned_count: usize,
}

pub fn prune_orphaned_mods(paths: &SzzPaths) -> Result<PruneSummary> {
    crate::instance::check_no_instance_running(paths)?;
    let reqs = crate::storage_gc::collect_active_requirements(paths)?;
    prune_orphaned_mods_with_hashes(paths, &reqs.mod_hashes)
}

/// Prunes orphaned mod JARs and stale .tmp files from the CAS store (~/.szz/store/)
/// given a pre-collected set of active mod hashes.
pub fn prune_orphaned_mods_with_hashes(
    paths: &SzzPaths,
    active_hashes: &std::collections::HashSet<String>,
) -> Result<PruneSummary> {
    crate::instance::check_no_instance_running(paths)?;

    let store_dir = paths.store_dir();
    if !store_dir.exists() {
        return Ok(PruneSummary {
            reclaimed_bytes: 0,
            pruned_count: 0,
        });
    }

    let mut reclaimed_bytes: u64 = 0;
    let mut pruned_count: usize = 0;

    let entries = std::fs::read_dir(&store_dir).map_err(|source| SzzError::Io {
        path: store_dir.clone(),
        source,
    })?;

    let now = std::time::SystemTime::now();

    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }

        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

        // Handle .tmp files: remove if older than 1 hour or broken
        if ext == "tmp" || file_name.contains(".tmp") {
            let should_remove = match meta.modified() {
                Ok(modified) => {
                    now.duration_since(modified)
                        .map(|d| d.as_secs() > 3600)
                        .unwrap_or(false)
                }
                Err(_) => true,
            };
            if should_remove {
                let len = meta.len();
                if std::fs::remove_file(&path).is_ok() {
                    reclaimed_bytes = reclaimed_bytes.saturating_add(len);
                    pruned_count += 1;
                }
            }
            continue;
        }

        // Handle .jar files: remove if orphaned
        if ext == "jar" {
            let stem = match path.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s.to_lowercase(),
                None => continue,
            };

            // If not used by any active instance and hardlink count is <= 1
            if !active_hashes.contains(&stem) {
                // If hardlink count fails to query, do not assume it is unlinked; skip deletion
                let Ok(nlink) = file_hardlink_count(&path) else {
                    continue;
                };
                if nlink <= 1 {
                    let len = meta.len();
                    if std::fs::remove_file(&path).is_ok() {
                        reclaimed_bytes = reclaimed_bytes.saturating_add(len);
                        pruned_count += 1;
                    }
                }
            }
        }
    }

    println!(
        "[szz-core] Pruned {} orphaned mod JARs ({} bytes reclaimed)",
        pruned_count, reclaimed_bytes
    );

    Ok(PruneSummary {
        reclaimed_bytes,
        pruned_count,
    })
}
