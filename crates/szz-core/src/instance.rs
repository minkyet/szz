use crate::error::{Result, SzzError};
use crate::manifest::ServerManifest;
use crate::paths::SzzPaths;
use crate::progress::{LaunchStep, ProgressReporter};
use crate::runtime::ensure_java_runtime;
use crate::store::sync_mods_with_reporter;
use crate::loaders::resolve_loader;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum InstanceLifecycleStatus {
    #[serde(alias = "notInstalled")]
    NotInstalled,
    #[serde(alias = "installing")]
    Installing,
    #[serde(alias = "installed")]
    Installed,
    #[serde(alias = "running")]
    Running,
    #[serde(alias = "updateAvailable")]
    UpdateAvailable,
    #[serde(alias = "updating")]
    Updating,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledInstanceInfo {
    pub instance_id: String,
    pub server_name: String,
    pub server_address: String,
    pub minecraft_version: String,
    pub loader: String,
    pub mod_count: usize,
    pub status: InstanceLifecycleStatus,
    pub installed_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceMetadata {
    pub instance_id: String,
    pub manifest: ServerManifest,
    pub installed_at: u64,
    pub last_played_at: Option<u64>,
}

impl InstanceMetadata {
    pub fn new(instance_id: impl Into<String>, manifest: ServerManifest) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            instance_id: instance_id.into(),
            manifest,
            installed_at: now,
            last_played_at: None,
        }
    }

    pub fn metadata_file_path(instance_dir: &Path) -> PathBuf {
        instance_dir.join(".szz_instance.json")
    }

    pub fn load_from_dir(instance_dir: &Path) -> Result<Option<Self>> {
        let path = Self::metadata_file_path(instance_dir);
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let meta: InstanceMetadata = serde_json::from_str(&content)?;
                Ok(Some(meta))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(SzzError::Io { path, source }),
        }
    }

    pub fn save_to_dir(&self, instance_dir: &Path) -> Result<()> {
        let path = Self::metadata_file_path(instance_dir);
        let temp_path = instance_dir.join(".szz_instance.json.tmp");
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&temp_path, &content).map_err(|source| SzzError::Io {
            path: temp_path.clone(),
            source,
        })?;
        std::fs::rename(&temp_path, &path).map_err(|source| SzzError::Io {
            path,
            source,
        })?;
        Ok(())
    }
}

/// Synchronizes all 5 vital resources for a server instance concurrently:
/// 1. Adoptium Temurin JRE runtime
/// 2. CAS Content-Addressable Storage mod JARs & OS hardlinks
/// 3. Loader libraries, intermediary mappings, and LWJGL natives
/// 4. Server configuration archive (config.zip) 2-stage staged transaction
/// 5. Mojang Piston Meta assets & Asset Index
pub async fn sync_instance_resources(
    paths: &SzzPaths,
    manifest: &ServerManifest,
    reporter: &dyn ProgressReporter,
    dry_run: bool,
) -> Result<(
    PathBuf,
    Vec<PathBuf>,
    crate::loaders::ResolvedLaunchMeta,
    Option<crate::config_sync::ConfigCommitHandle>,
    crate::assets::AssetSyncResult,
)> {
    reporter.report(LaunchStep::PreparingInstance, None);
    println!("[szz-core] === Preparing instance directories for '{}' ===", manifest.server_id);
    paths.ensure_instance_dirs(&manifest.server_id)?;

    reporter.report(
        LaunchStep::CheckingJavaRuntime,
        Some("Synchronizing runtime, mods, and libraries concurrently..."),
    );
    let java_version = manifest.required_java_version();
    let java_fut = ensure_java_runtime(paths, java_version);
    let mods_fut = sync_mods_with_reporter(paths, &manifest.server_id, &manifest.mods, reporter);
    let loader_fut = resolve_loader(paths, &manifest.server_id, manifest);
    let config_fut = crate::config_sync::sync_server_config(paths, manifest, dry_run);
    let assets_fut = crate::assets::sync_mojang_assets(paths, &manifest.minecraft_version, reporter, dry_run);

    tokio::try_join!(java_fut, mods_fut, loader_fut, config_fut, assets_fut)
}

/// Validates an instance ID against directory traversal and invalid characters.
/// Instance IDs must not be empty and must only contain ASCII alphanumeric characters, hyphens, or underscores.
pub fn validate_instance_id(instance_id: &str) -> Result<()> {
    if instance_id.trim().is_empty()
        || !instance_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(SzzError::Security(format!(
            "Invalid instance ID '{}': must not be empty and must match ^[a-zA-Z0-9_-]+$",
            instance_id
        )));
    }
    Ok(())
}

/// Pre-installs an instance from a ServerManifest without spawning the Minecraft game process.
/// Synchronizes all 5 vital resources and commits the instance metadata (`.szz_instance.json`).
pub async fn install_instance(
    paths: &SzzPaths,
    manifest: &ServerManifest,
    reporter: &dyn ProgressReporter,
    dry_run: bool,
) -> Result<()> {
    validate_instance_id(&manifest.server_id)?;
    if crate::storage_gc::is_gc_in_progress() {
        return Err(SzzError::StorageGcInProgress);
    }
    let instance_dir = paths.instance_dir(&manifest.server_id);

    let (_java_binary, linked_mods, resolved, config_commit_opt, asset_sync) =
        sync_instance_resources(paths, manifest, reporter, dry_run).await?;

    if let Some(commit_handle) = config_commit_opt {
        if !dry_run {
            commit_handle.commit()?;
        }
    }

    if !dry_run {
        let meta = InstanceMetadata::new(&manifest.server_id, manifest.clone());
        meta.save_to_dir(&instance_dir)?;
    }

    reporter.report(
        LaunchStep::Complete,
        Some(&format!(
            "Instance '{}' pre-installed successfully ({} mods, {} libraries, asset index '{}')",
            manifest.server_id,
            linked_mods.len(),
            resolved.library_count,
            asset_sync.asset_index_id
        )),
    );
    println!(
        "[szz-core] Instance '{}' installed successfully at {:?}",
        manifest.server_id, instance_dir
    );

    Ok(())
}

/// Uninstalls an instance by deleting its local game directory (~/.szz/instances/<instance_id>/).
/// Rejects deletion if the instance process is currently running.
pub fn uninstall_instance(paths: &SzzPaths, instance_id: &str) -> Result<()> {
    validate_instance_id(instance_id)?;
    if crate::storage_gc::is_gc_in_progress() {
        return Err(SzzError::StorageGcInProgress);
    }
    let status = get_instance_status(paths, instance_id)?;
    if status == InstanceLifecycleStatus::Running {
        return Err(SzzError::InstanceRunning(instance_id.to_string()));
    }

    let instance_dir = paths.instance_dir(instance_id);
    if instance_dir.exists() {
        std::fs::remove_dir_all(&instance_dir).map_err(|source| SzzError::Io {
            path: instance_dir.clone(),
            source,
        })?;
        println!("[szz-core] Instance '{}' uninstalled successfully", instance_id);
    }

    // Automatically prune orphaned mods from the CAS store
    if let Err(err) = crate::store::prune_orphaned_mods(paths) {
        eprintln!("[szz-core] Warning: post-uninstall mod prune failed: {}", err);
    }

    Ok(())
}

#[cfg(unix)]
pub fn is_pid_alive(pid: u32) -> bool {
    let res = unsafe { libc::kill(pid as libc::pid_t, 0) };
    if res == 0 {
        true
    } else {
        std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }
}

#[cfg(not(unix))]
pub fn is_pid_alive(_pid: u32) -> bool {
    false
}

/// Verifies that no server instance is currently running via PID file inspection.
/// Cleans up any stale PID files whose processes are no longer alive.
pub fn check_no_instance_running(paths: &SzzPaths) -> Result<()> {
    let instances_dir = paths.instances_dir();
    if !instances_dir.exists() {
        return Ok(());
    }

    let entries = std::fs::read_dir(&instances_dir).map_err(|source| SzzError::Io {
        path: instances_dir.clone(),
        source,
    })?;

    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            let pid_file = p.join(".szz_instance.pid");
            if pid_file.exists() {
                if let Ok(content) = std::fs::read_to_string(&pid_file) {
                    if let Ok(pid) = content.trim().parse::<u32>() {
                        if is_pid_alive(pid) {
                            let id = p.file_name().and_then(|s| s.to_str()).unwrap_or("unknown");
                            return Err(SzzError::InstanceRunning(id.to_string()));
                        }
                    }
                }
                // Stale PID file cleanup
                let _ = std::fs::remove_file(&pid_file);
            }
        }
    }

    Ok(())
}

/// Returns the lifecycle status of an instance.
pub fn get_instance_status(paths: &SzzPaths, instance_id: &str) -> Result<InstanceLifecycleStatus> {
    validate_instance_id(instance_id)?;
    let instance_dir = paths.instance_dir(instance_id);
    if !instance_dir.exists() {
        return Ok(InstanceLifecycleStatus::NotInstalled);
    }

    let meta_file = InstanceMetadata::metadata_file_path(&instance_dir);
    if !meta_file.exists() {
        return Ok(InstanceLifecycleStatus::NotInstalled);
    }

    // Check if process is actively running via PID file
    let pid_file = instance_dir.join(".szz_instance.pid");
    if pid_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&pid_file) {
            if let Ok(pid) = content.trim().parse::<u32>() {
                if is_pid_alive(pid) {
                    return Ok(InstanceLifecycleStatus::Running);
                }
            }
        }
        // Stale PID file: cleanup
        let _ = std::fs::remove_file(&pid_file);
    }

    Ok(InstanceLifecycleStatus::Installed)
}

/// Scans ~/.szz/instances/ and returns all installed instances with their metadata and status.
pub fn list_installed_instances(paths: &SzzPaths) -> Result<Vec<InstalledInstanceInfo>> {
    let instances_dir = paths.instances_dir();
    if !instances_dir.exists() {
        return Ok(Vec::new());
    }

    let entries = std::fs::read_dir(&instances_dir).map_err(|source| SzzError::Io {
        path: instances_dir.clone(),
        source,
    })?;

    let mut result = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| SzzError::Io {
            path: instances_dir.clone(),
            source,
        })?;

        if !entry.path().is_dir() {
            continue;
        }

        let instance_dir = entry.path();
        if let Some(meta) = InstanceMetadata::load_from_dir(&instance_dir)? {
            let loader_str = format!(
                "{:?} {}",
                meta.manifest.loader.loader_type, meta.manifest.loader.version
            );
            let status = get_instance_status(paths, &meta.instance_id)?;

            result.push(InstalledInstanceInfo {
                instance_id: meta.instance_id,
                server_name: meta.manifest.name,
                server_address: meta.manifest.server_address,
                minecraft_version: meta.manifest.minecraft_version,
                loader: loader_str,
                mod_count: meta.manifest.mods.len(),
                status,
                installed_at: meta.installed_at,
            });
        }
    }

    result.sort_by(|a, b| a.instance_id.cmp(&b.instance_id));
    Ok(result)
}

/// Represents the diff between an installed instance manifest and a target manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceUpdateDiff {
    pub added_mods: Vec<crate::manifest::ModEntry>,
    pub removed_mods: Vec<crate::manifest::ModEntry>,
    pub unchanged_mods: Vec<crate::manifest::ModEntry>,
    pub config_needs_update: bool,
    pub has_changes: bool,
    pub old_manifest_version: u32,
    pub new_manifest_version: u32,
}

/// Computes the diff between an installed instance manifest and a target update manifest.
pub fn check_instance_update(
    installed_manifest: &ServerManifest,
    target_manifest: &ServerManifest,
) -> InstanceUpdateDiff {
    let mut added_mods = Vec::new();
    let mut unchanged_mods = Vec::new();

    let mut installed_map: std::collections::HashMap<String, &crate::manifest::ModEntry> =
        std::collections::HashMap::new();
    for m in &installed_manifest.mods {
        installed_map.insert(m.identity_key(), m);
    }

    let mut target_keys = std::collections::HashSet::new();
    for m in &target_manifest.mods {
        let key = m.identity_key();
        target_keys.insert(key.clone());

        if installed_map.contains_key(&key) {
            unchanged_mods.push(m.clone());
        } else {
            added_mods.push(m.clone());
        }
    }

    let mut removed_mods = Vec::new();
    for (key, m) in &installed_map {
        if !target_keys.contains(key) {
            removed_mods.push((*m).clone());
        }
    }

    let config_needs_update = match (&installed_manifest.config, &target_manifest.config) {
        (None, None) => false,
        (None, Some(_)) => true,
        (Some(_), None) => false,
        (Some(old_cfg), Some(new_cfg)) => {
            if old_cfg != new_cfg {
                true
            } else if old_cfg.hash.is_some() {
                false
            } else {
                target_manifest.manifest_version > installed_manifest.manifest_version
            }
        }
    };

    let has_changes = !added_mods.is_empty()
        || !removed_mods.is_empty()
        || config_needs_update
        || target_manifest.manifest_version > installed_manifest.manifest_version;

    InstanceUpdateDiff {
        added_mods,
        removed_mods,
        unchanged_mods,
        config_needs_update,
        has_changes,
        old_manifest_version: installed_manifest.manifest_version,
        new_manifest_version: target_manifest.manifest_version,
    }
}

/// Evaluates instance status taking remote catalog `latest_manifest_version` into account.
/// If installed and remote manifest version is strictly greater, returns `InstanceLifecycleStatus::UpdateAvailable`.
pub fn get_instance_status_with_remote(
    paths: &SzzPaths,
    instance_id: &str,
    latest_manifest_version: Option<u32>,
) -> Result<InstanceLifecycleStatus> {
    let local_status = get_instance_status(paths, instance_id)?;
    if local_status == InstanceLifecycleStatus::Installed {
        if let Some(target_ver) = latest_manifest_version {
            let instance_dir = paths.instance_dir(instance_id);
            if let Ok(Some(meta)) = InstanceMetadata::load_from_dir(&instance_dir) {
                if meta.manifest.manifest_version < target_ver {
                    return Ok(InstanceLifecycleStatus::UpdateAvailable);
                }
            }
        }
    }
    Ok(local_status)
}

/// Incrementally updates an installed instance to match `target_manifest`.
///
/// Only downloads added/modified mods into CAS, unlinks removed mods from `mods/`,
/// cleans up orphaned mods, applies config updates while preserving user shaders/options,
/// and atomically commits the updated manifest to `.szz_instance.json` upon completion.
pub async fn update_instance(
    paths: &SzzPaths,
    target_manifest: &ServerManifest,
    reporter: &dyn ProgressReporter,
) -> Result<InstanceUpdateDiff> {
    target_manifest.validate()?;
    validate_instance_id(&target_manifest.server_id)?;
    if crate::storage_gc::is_gc_in_progress() {
        return Err(SzzError::StorageGcInProgress);
    }
    let status = get_instance_status(paths, &target_manifest.server_id)?;
    if status == InstanceLifecycleStatus::Running {
        return Err(SzzError::InstanceRunning(target_manifest.server_id.clone()));
    }
    if status == InstanceLifecycleStatus::NotInstalled {
        return Err(SzzError::InstanceNotInstalled(target_manifest.server_id.clone()));
    }

    let instance_dir = paths.instance_dir(&target_manifest.server_id);
    let installed_meta = InstanceMetadata::load_from_dir(&instance_dir)?
        .ok_or_else(|| SzzError::InstanceNotInstalled(target_manifest.server_id.clone()))?;

    let diff = check_instance_update(&installed_meta.manifest, target_manifest);

    reporter.report(
        LaunchStep::SyncingMods,
        Some(&format!(
            "Updating instance '{}': +{} mods, -{} mods",
            target_manifest.server_id,
            diff.added_mods.len(),
            diff.removed_mods.len()
        )),
    );

    let target_filenames: std::collections::HashSet<String> = target_manifest
        .mods
        .iter()
        .map(|m| m.filename())
        .collect();

    // 1. Unlink removed mods safely (only those NOT retained by target_manifest)
    let mods_dir = paths.instance_mods_dir(&target_manifest.server_id);
    for m in &diff.removed_mods {
        let filename = m.filename();
        if target_filenames.contains(&filename) {
            // In-place mod upgrade with identical destination filename: will be overwritten by added_mods
            continue;
        }
        if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
            continue;
        }
        let target_file = mods_dir.join(filename);
        if target_file.starts_with(&mods_dir) && target_file.is_file() {
            let _ = std::fs::remove_file(&target_file);
        }
    }

    // 2. Download added/modified mods and create hardlinks
    if !diff.added_mods.is_empty() {
        sync_mods_with_reporter(paths, &target_manifest.server_id, &diff.added_mods, reporter).await?;
    }

    // 3. Update config archive if needed
    if diff.config_needs_update && target_manifest.config.is_some() {
        reporter.report(
            LaunchStep::PreparingInstance,
            Some("Applying updated server config..."),
        );
        let commit_handle = crate::config_sync::sync_server_config(paths, target_manifest, false).await?;
        if let Some(handle) = commit_handle {
            handle.commit()?;
        }
    }

    // 4. Atomic metadata commit (only after all download, linking, and config steps succeed)
    let mut updated_meta = installed_meta;
    updated_meta.manifest = target_manifest.clone();
    updated_meta.save_to_dir(&instance_dir)?;

    // 5. Prune orphaned mods from CAS (must execute after metadata commit so active requirements reflect new manifest)
    if !diff.removed_mods.is_empty() {
        if let Err(err) = crate::store::prune_orphaned_mods(paths) {
            eprintln!("[szz-core] Warning: post-update mod prune failed: {}", err);
        }
    }

    reporter.report(
        LaunchStep::Complete,
        Some(&format!(
            "Instance '{}' updated successfully to manifest v{}",
            target_manifest.server_id,
            target_manifest.manifest_version
        )),
    );

    Ok(diff)
}
