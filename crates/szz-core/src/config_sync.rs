use crate::error::{Result, SzzError};
use crate::manifest::{ConfigSpec, HashAlgorithm};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ConfigState {
    pub version: Option<String>,
    pub hash: Option<String>,
    pub synced_at: u64,
    #[serde(default)]
    pub managed_files: Vec<String>,
}

impl ConfigState {
    pub fn load_from_dir(instance_dir: &Path) -> Result<Option<Self>> {
        let state_path = instance_dir.join(".szz_config_state.json");
        if !state_path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&state_path).map_err(|source| SzzError::Io {
            path: state_path.clone(),
            source,
        })?;
        let state: ConfigState = serde_json::from_str(&content)?;
        Ok(Some(state))
    }

    pub fn save_to_dir(&self, instance_dir: &Path) -> Result<()> {
        let state_path = instance_dir.join(".szz_config_state.json");
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&state_path, content).map_err(|source| SzzError::Io {
            path: state_path,
            source,
        })?;
        Ok(())
    }
}

pub struct ConfigCommitHandle {
    pub state: ConfigState,
    pub instance_dir: PathBuf,
}

impl ConfigCommitHandle {
    pub fn commit(self) -> Result<()> {
        self.state.save_to_dir(&self.instance_dir)
    }
}

pub fn is_dangerous_executable(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_ascii_lowercase();
        matches!(
            ext_lower.as_str(),
            "exe" | "bat" | "cmd" | "sh" | "ps1" | "vbs" | "scr" | "pif" | "com" | "dll" | "so" | "dylib"
        )
    } else {
        false
    }
}

pub fn is_sensitive_system_path(path: &Path) -> bool {
    for comp in path.components() {
        let s = comp.as_os_str().to_string_lossy();
        if s == ".git"
            || s.starts_with(".szz")
            || s == "servers.dat"
            || s == "launcher_profiles.json"
            || s == "usercache.json"
        {
            return true;
        }
    }
    if let Some(first) = path.iter().next() {
        let s = first.to_string_lossy();
        if s.starts_with('.') && s != "." && s != ".." {
            return true;
        }
    }
    false
}

pub fn is_protected_user_path(path: &Path) -> bool {
    for comp in path.components() {
        let s = comp.as_os_str().to_string_lossy();
        let s_lower = s.to_ascii_lowercase();
        if s_lower == "saves"
            || s_lower == "screenshots"
            || s_lower == "logs"
            || s_lower == "servers.dat"
            || s_lower == "xaerominimap"
            || s_lower == "journeymap"
        {
            return true;
        }
    }
    false
}

/// Identifies user-customizable configuration files (e.g. keybindings, graphics, shader settings)
/// that should be preserved across server config updates once initialized.
pub fn is_user_customizable_config(path: &Path) -> bool {
    let s = path.to_string_lossy();
    let s_normalized = if s.contains('\\') {
        s.replace('\\', "/")
    } else {
        s.to_string()
    };
    let s_lower = s_normalized.to_ascii_lowercase();
    let s_clean = s_lower
        .trim_start_matches("./")
        .trim_start_matches('/');

    // 1. Root game options & shader options
    if s_clean == "options.txt" || s_clean == "optionsshaders.txt" {
        return true;
    }

    // 2. Core renderer and shader engine configs (Iris / Sodium)
    if s_clean == "config/iris.properties" || s_clean == "config/sodium-options.json" {
        return true;
    }

    // 3. Shaderpack configuration files (.txt, .properties inside shaderpacks/)
    if s_clean.starts_with("shaderpacks/")
        && (s_clean.ends_with(".txt") || s_clean.ends_with(".properties"))
    {
        return true;
    }

    false
}

/// Applies a configuration archive to the server instance directory.
///
/// Steps:
/// 1. Unpacks non-excluded files into `~/.szz/servers/<id>/.tmp_extract/`.
/// 2. Performs security checks (Zip-Slip, symlink rejection, executable blocking, CAS protection).
/// 3. If dry-run, discards staging and returns a dry commit handle.
/// 4. If real run and instance has existing overrides, snapshots them into `.szz_backups/overrides_<timestamp>/`.
/// 5. Prunes deleted managed files from previous installations.
/// 6. Promotes extracted files from staging into the instance directory.
/// 7. Cleans up staging directory and returns a commit handle.
pub fn apply_config_archive(
    zip_bytes: &[u8],
    instance_dir: &Path,
    config_spec: &ConfigSpec,
    dry_run: bool,
) -> Result<ConfigCommitHandle> {
    if !instance_dir.exists() {
        std::fs::create_dir_all(instance_dir).map_err(|source| SzzError::Io {
            path: instance_dir.to_path_buf(),
            source,
        })?;
    }

    let old_state = ConfigState::load_from_dir(instance_dir)?;
    let is_first_install = old_state.is_none() && !instance_dir.join("options.txt").exists();

    let staging_dir = instance_dir.join(".tmp_extract");
    if staging_dir.exists() {
        let _ = std::fs::remove_dir_all(&staging_dir);
    }
    std::fs::create_dir_all(&staging_dir).map_err(|source| SzzError::Io {
        path: staging_dir.clone(),
        source,
    })?;

    let cursor = std::io::Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| SzzError::Config(format!("Failed to read zip archive: {}", e)))?;

    let mut extracted_files: Vec<String> = Vec::new();

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| SzzError::Config(format!("Failed to read zip entry #{}: {}", i, e)))?;

        // 1. Symlink / Hardlink check
        if entry.is_symlink() || entry.unix_mode().is_some_and(|mode| (mode & 0o170000) == 0o120000) {
            let _ = std::fs::remove_dir_all(&staging_dir);
            return Err(SzzError::Config(format!(
                "Symbolic or hard links in config archive are strictly rejected for security: {}",
                entry.name()
            )));
        }

        // 2. Zip-Slip path traversal check
        let enclosed = entry.enclosed_name().ok_or_else(|| {
            let _ = std::fs::remove_dir_all(&staging_dir);
            SzzError::Config(format!("Zip-slip traversal detected in entry: {}", entry.name()))
        })?;

        // 3. Strip 'overrides/' prefix if present
        let rel_path = match enclosed.strip_prefix("overrides") {
            Ok(p) => p,
            Err(_) => enclosed.as_path(),
        };

        if rel_path.as_os_str().is_empty() {
            continue;
        }

        // 4. CAS integrity check: reject writing directly into mods/ or libraries/
        if rel_path.starts_with("mods") || rel_path.starts_with("libraries") {
            continue;
        }

        // 5. Executable & sensitive path security guards
        if is_dangerous_executable(rel_path) || is_sensitive_system_path(rel_path) {
            continue;
        }

        // 6. User protected paths check (saves, screenshots, logs, etc.)
        if is_protected_user_path(rel_path) && instance_dir.join(rel_path).exists() {
            continue;
        }

        // 7. User customizable config smart lifecycle: preserve user options/shaders on update
        if is_user_customizable_config(rel_path)
            && !is_first_install
            && instance_dir.join(rel_path).exists()
        {
            continue;
        }

        let dest = staging_dir.join(rel_path);

        if entry.is_dir() {
            std::fs::create_dir_all(&dest).map_err(|source| SzzError::Io {
                path: dest,
                source,
            })?;
        } else {
            if let Some(parent) = dest.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent).map_err(|source| SzzError::Io {
                        path: parent.to_path_buf(),
                        source,
                    })?;
                }
            }

            let mut out = std::fs::File::create(&dest).map_err(|source| SzzError::Io {
                path: dest.clone(),
                source,
            })?;

            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(|source| SzzError::Io {
                path: dest.clone(),
                source,
            })?;

            use std::io::Write;
            out.write_all(&buf).map_err(|source| SzzError::Io {
                path: dest.clone(),
                source,
            })?;

            let rel_str = rel_path.to_string_lossy().replace('\\', "/");
            extracted_files.push(rel_str);
        }
    }

    if dry_run {
        let _ = std::fs::remove_dir_all(&staging_dir);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        return Ok(ConfigCommitHandle {
            state: ConfigState {
                version: config_spec.version.clone(),
                hash: config_spec.hash.clone(),
                synced_at: now,
                managed_files: extracted_files,
            },
            instance_dir: instance_dir.to_path_buf(),
        });
    }

    // Snapshot existing overrides if updating
    create_overrides_backup(instance_dir, 3)?;

    // Prune stale managed files from previous installation
    if let Some(old) = &old_state {
        prune_stale_managed_files(instance_dir, old, &extracted_files)?;
    }

    // Promote files from staging into instance directory
    promote_staging_files(&staging_dir, instance_dir)?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    Ok(ConfigCommitHandle {
        state: ConfigState {
            version: config_spec.version.clone(),
            hash: config_spec.hash.clone(),
            synced_at: now,
            managed_files: extracted_files,
        },
        instance_dir: instance_dir.to_path_buf(),
    })
}

pub fn create_overrides_backup(instance_dir: &Path, max_backups: usize) -> Result<Option<PathBuf>> {
    if !instance_dir.exists() {
        return Ok(None);
    }

    let backups_dir = instance_dir.join(".szz_backups");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let current_backup = backups_dir.join(format!("overrides_{}", now));

    let mut copied_any = false;
    for entry in std::fs::read_dir(instance_dir).map_err(|source| SzzError::Io {
        path: instance_dir.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| SzzError::Io {
            path: instance_dir.to_path_buf(),
            source,
        })?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if name_str == "mods"
            || name_str == ".szz_backups"
            || name_str == ".tmp_extract"
            || name_str == "saves"
            || name_str == "screenshots"
            || name_str == "logs"
            || name_str == ".szz_config_state.json"
        {
            continue;
        }

        let ft = entry.file_type().map_err(|source| SzzError::Io {
            path: entry.path(),
            source,
        })?;

        if ft.is_dir() {
            let dest_dir = current_backup.join(&name);
            copy_dir_recursive(&entry.path(), &dest_dir)?;
            copied_any = true;
        } else if ft.is_file() && !is_user_customizable_config(Path::new(&name)) {
            // Also back up top-level overrides files if any (excluding user customizable configs preserved anyway)
            if !current_backup.exists() {
                std::fs::create_dir_all(&current_backup).map_err(|source| SzzError::Io {
                    path: current_backup.clone(),
                    source,
                })?;
            }
            let dest_file = current_backup.join(&name);
            std::fs::copy(entry.path(), &dest_file).map_err(|source| SzzError::Io {
                path: dest_file,
                source,
            })?;
            copied_any = true;
        }
    }

    if !copied_any {
        return Ok(None);
    }

    prune_old_backups(&backups_dir, max_backups)?;
    Ok(Some(current_backup))
}

pub fn prune_stale_managed_files(
    instance_dir: &Path,
    old_state: &ConfigState,
    new_files: &[String],
) -> Result<Vec<PathBuf>> {
    let new_set: HashSet<&str> = new_files.iter().map(|s| s.as_str()).collect();
    let mut removed = Vec::new();

    for old_file in &old_state.managed_files {
        if !new_set.contains(old_file.as_str()) {
            let rel = Path::new(old_file);
            if is_protected_user_path(rel) || is_user_customizable_config(rel) {
                continue;
            }
            let target = instance_dir.join(rel);
            if target.is_file() && std::fs::remove_file(&target).is_ok() {
                removed.push(target.clone());
                if let Some(parent) = target.parent() {
                    let _ = remove_empty_parents(parent, instance_dir);
                }
            }
        }
    }
    Ok(removed)
}

fn remove_empty_parents(dir: &Path, stop_at: &Path) -> std::io::Result<()> {
    let mut current = dir.to_path_buf();
    while current != stop_at {
        if let Ok(mut read) = std::fs::read_dir(&current) {
            if read.next().is_none() {
                std::fs::remove_dir(&current)?;
                if let Some(parent) = current.parent() {
                    current = parent.to_path_buf();
                    continue;
                }
            }
        }
        break;
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst).map_err(|source| SzzError::Io {
            path: dst.to_path_buf(),
            source,
        })?;
    }
    for entry in std::fs::read_dir(src).map_err(|source| SzzError::Io {
        path: src.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| SzzError::Io {
            path: src.to_path_buf(),
            source,
        })?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let file_type = entry.file_type().map_err(|source| SzzError::Io {
            path: src_path.clone(),
            source,
        })?;
        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            std::fs::copy(&src_path, &dst_path).map_err(|source| SzzError::Io {
                path: dst_path,
                source,
            })?;
        }
    }
    Ok(())
}

fn prune_old_backups(backups_dir: &Path, max_backups: usize) -> Result<()> {
    if !backups_dir.exists() {
        return Ok(());
    }
    let mut backups: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(backups_dir).map_err(|source| SzzError::Io {
        path: backups_dir.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| SzzError::Io {
            path: backups_dir.to_path_buf(),
            source,
        })?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("overrides_") && entry.path().is_dir() {
            backups.push(entry.path());
        }
    }

    backups.sort();

    if backups.len() > max_backups {
        let to_remove = backups.len() - max_backups;
        for path in backups.iter().take(to_remove) {
            let _ = std::fs::remove_dir_all(path);
        }
    }
    Ok(())
}

fn promote_staging_files(staging_dir: &Path, instance_dir: &Path) -> Result<()> {
    if !staging_dir.exists() {
        return Ok(());
    }
    promote_dir_recursive(staging_dir, staging_dir, instance_dir)?;
    let _ = std::fs::remove_dir_all(staging_dir);
    Ok(())
}

fn promote_dir_recursive(root: &Path, current: &Path, target_root: &Path) -> Result<()> {
    for entry in std::fs::read_dir(current).map_err(|source| SzzError::Io {
        path: current.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| SzzError::Io {
            path: current.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let rel = path
            .strip_prefix(root)
            .map_err(|e| SzzError::Config(e.to_string()))?;
        let target = target_root.join(rel);

        let ft = entry.file_type().map_err(|source| SzzError::Io {
            path: path.clone(),
            source,
        })?;
        if ft.is_dir() {
            if !target.exists() {
                std::fs::create_dir_all(&target).map_err(|source| SzzError::Io {
                    path: target.clone(),
                    source,
                })?;
            }
            promote_dir_recursive(root, &path, target_root)?;
        } else if ft.is_file() {
            if let Some(parent) = target.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent).map_err(|source| SzzError::Io {
                        path: parent.to_path_buf(),
                        source,
                    })?;
                }
            }
            if std::fs::rename(&path, &target).is_err() {
                std::fs::copy(&path, &target).map_err(|source| SzzError::Io {
                    path: target.clone(),
                    source,
                })?;
                let _ = std::fs::remove_file(&path);
            }
        }
    }
    Ok(())
}

pub fn verify_bytes_hash(bytes: &[u8], expected: &str) -> Result<()> {
    let (clean_expected, algo) = HashAlgorithm::parse_spec(expected).ok_or_else(|| {
        SzzError::Config(format!(
            "Unsupported hash format: '{}'. Expected SHA-1 (40 chars) or SHA-512 (128 chars).",
            expected
        ))
    })?;

    if !algo.verify_bytes(bytes, clean_expected) {
        return Err(SzzError::Config(format!(
            "Config archive hash mismatch! Expected: {}",
            clean_expected
        )));
    }
    Ok(())
}

fn resolve_local_config_file(url: &str) -> Option<PathBuf> {
    let clean_url = url.trim().strip_prefix("file://").unwrap_or(url.trim());
    let path = Path::new(clean_url);

    // 1. Direct path exists
    if path.is_file() {
        return Some(path.to_path_buf());
    }

    // 2. Relative to current working dir in common fixture locations
    if let Some(filename) = path.file_name() {
        let candidates = [
            PathBuf::from("fixtures/servers/config").join(filename),
            PathBuf::from("szz/fixtures/servers/config").join(filename),
            PathBuf::from("fixtures/config").join(filename),
            PathBuf::from("szz/fixtures/config").join(filename),
        ];

        for candidate in &candidates {
            if candidate.is_file() {
                return Some(candidate.clone());
            }
        }

        // 3. Relative to compile-time CARGO_MANIFEST_DIR (for tests)
        if let Some(manifest_dir) = option_env!("CARGO_MANIFEST_DIR") {
            let manifest_path = PathBuf::from(manifest_dir);
            let cargo_candidates = [
                manifest_path.join("../../fixtures/servers/config").join(filename),
                manifest_path.join("../fixtures/servers/config").join(filename),
                manifest_path.join("fixtures/servers/config").join(filename),
            ];
            for candidate in &cargo_candidates {
                if candidate.is_file() {
                    return Some(candidate.clone());
                }
            }
        }
    }

    None
}

pub async fn sync_server_config(
    paths: &crate::paths::SzzPaths,
    manifest: &crate::manifest::ServerManifest,
    dry_run: bool,
) -> Result<Option<ConfigCommitHandle>> {
    let Some(config_spec) = &manifest.config else {
        return Ok(None);
    };

    let is_remote_http = config_spec.url.starts_with("http://") || config_spec.url.starts_with("https://");

    let bytes = if is_remote_http {
        if dry_run {
            return Ok(None);
        }

        println!("[szz-core] Downloading and synchronizing server configuration package from '{}'...", config_spec.url);
        let client = reqwest::Client::new();
        let response = client
            .get(&config_spec.url)
            .send()
            .await
            .map_err(|source| SzzError::Network {
                url: config_spec.url.clone(),
                source,
            })?;

        if !response.status().is_success() {
            return Err(SzzError::Config(format!(
                "Failed to download config archive from {}: HTTP {}",
                config_spec.url,
                response.status()
            )));
        }

        response
            .bytes()
            .await
            .map_err(|source| SzzError::Network {
                url: config_spec.url.clone(),
                source,
            })?
            .to_vec()
    } else if let Some(local_path) = resolve_local_config_file(&config_spec.url) {
        println!("[szz-core] Loading local server configuration package from '{}'...", local_path.display());
        std::fs::read(&local_path).map_err(|source| SzzError::Io {
            path: local_path,
            source,
        })?
    } else if dry_run {
        return Ok(None);
    } else {
        return Err(SzzError::Config(format!(
            "Config archive '{}' is neither an HTTP URL nor an existing local file",
            config_spec.url
        )));
    };

    if let Some(expected_hash) = &config_spec.hash {
        verify_bytes_hash(&bytes, expected_hash)?;
    }

    let instance_dir = paths.instance_dir(&manifest.server_id);
    let config_spec_clone = config_spec.clone();

    let commit_handle = tokio::task::spawn_blocking(move || {
        apply_config_archive(&bytes, &instance_dir, &config_spec_clone, dry_run)
    })
    .await
    .map_err(|e| SzzError::Config(format!("Blocking task execution error: {}", e)))??;

    Ok(Some(commit_handle))
}
