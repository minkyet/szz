use crate::error::{Result, SzzError};
use crate::paths::SzzPaths;
use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

static GC_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

/// Returns true if a storage GC operation is currently in progress.
pub fn is_gc_in_progress() -> bool {
    GC_IN_PROGRESS.load(Ordering::SeqCst)
}

/// RAII lock guard to ensure only one GC operation runs at a time
/// and that game launches are blocked while GC is active.
#[derive(Debug)]
pub struct GcLockGuard;

impl GcLockGuard {
    pub fn acquire() -> Result<Self> {
        if GC_IN_PROGRESS
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(SzzError::StorageGcInProgress);
        }
        Ok(Self)
    }
}

impl Drop for GcLockGuard {
    fn drop(&mut self) {
        GC_IN_PROGRESS.store(false, Ordering::SeqCst);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageCategoryUsage {
    pub total_bytes: u64,
    pub reclaimable_bytes: u64,
    pub item_count: usize,
    pub reclaimable_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageUsageReport {
    pub total_bytes: u64,
    pub reclaimable_bytes: u64,
    pub mods: StorageCategoryUsage,
    pub runtimes: StorageCategoryUsage,
    pub assets: StorageCategoryUsage,
    pub libraries: StorageCategoryUsage,
    pub instances: StorageCategoryUsage,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanOptions {
    #[serde(default)]
    pub clean_mods: bool,
    #[serde(default)]
    pub clean_runtimes: bool,
    #[serde(default)]
    pub clean_assets: bool,
    #[serde(default)]
    pub clean_libraries: bool,
    #[serde(default)]
    pub dry_run: bool,
}

impl CleanOptions {
    pub fn all() -> Self {
        Self {
            clean_mods: true,
            clean_runtimes: true,
            clean_assets: true,
            clean_libraries: true,
            dry_run: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanSummary {
    pub reclaimed_bytes: u64,
    pub deleted_mods_count: usize,
    pub deleted_runtimes_count: usize,
    pub deleted_assets_count: usize,
    pub deleted_libraries_count: usize,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ActiveRequirements {
    pub mod_hashes: HashSet<String>,
    pub java_versions: HashSet<u32>,
    pub mc_versions: HashSet<String>,
    pub asset_index_ids: HashSet<String>,
    pub instance_count: usize,
}

/// Recursively calculates the byte size and file count of a directory.
pub fn calculate_dir_size(path: &Path) -> (u64, usize) {
    let Ok(meta) = path.metadata() else {
        return (0, 0);
    };
    if meta.is_file() {
        return (meta.len(), 1);
    }

    let mut total_bytes: u64 = 0;
    let mut count: usize = 0;
    let mut stack = vec![path.to_path_buf()];

    while let Some(current) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(current) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_symlink() {
                        continue;
                    }
                    if file_type.is_dir() {
                        stack.push(entry.path());
                    } else if file_type.is_file() {
                        count += 1;
                        total_bytes = total_bytes.saturating_add(entry.metadata().map(|m| m.len()).unwrap_or(0));
                    }
                }
            }
        }
    }
    (total_bytes, count)
}

/// Validates that a directory name strictly matches `java-<major_version>` (e.g. `java-8`, `java-17`).
pub fn parse_java_runtime_dir_version(name: &str) -> Option<u32> {
    if let Some(rest) = name.strip_prefix("java-") {
        if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
            return rest.parse::<u32>().ok();
        }
    }
    None
}

/// Helper predicate to check if an asset index ID matches any active index requirements.
#[inline]
pub fn is_asset_index_active(id: &str, active_index_ids: &HashSet<String>) -> bool {
    active_index_ids.contains(id)
        || active_index_ids.iter().any(|aid| aid.starts_with(id) || id.starts_with(aid))
}

/// Scans installed instances and derives all active requirements.
pub fn collect_active_requirements(paths: &SzzPaths) -> Result<ActiveRequirements> {
    let mut reqs = ActiveRequirements::default();
    let instances_dir = paths.instances_dir();

    if !instances_dir.exists() {
        return Ok(reqs);
    }

    // Map existing CAS store files by inode on Unix for instant O(1) hash resolution without reading file contents
    #[cfg(unix)]
    let mut store_inodes: std::collections::HashMap<u64, String> = std::collections::HashMap::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let Ok(store_entries) = std::fs::read_dir(paths.store_dir()) {
            for entry in store_entries.flatten() {
                let p = entry.path();
                if p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("jar") {
                    if let Ok(meta) = entry.metadata() {
                        if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                            store_inodes.insert(meta.ino(), stem.to_lowercase());
                        }
                    }
                }
            }
        }
    }

    let entries = std::fs::read_dir(&instances_dir).map_err(|source| SzzError::Io {
        path: instances_dir.clone(),
        source,
    })?;
    for entry in entries.flatten() {
        let inst_dir = entry.path();
        if !inst_dir.is_dir() {
            continue;
        }

        reqs.instance_count += 1;
        let mut loaded_manifest = false;

        if let Ok(Some(meta)) = crate::instance::InstanceMetadata::load_from_dir(&inst_dir) {
            reqs.java_versions.insert(meta.manifest.required_java_version());
            let is_new_mc_version = reqs.mc_versions.insert(meta.manifest.minecraft_version.clone());

            for m in &meta.manifest.mods {
                if let Some(s) = &m.sha1 {
                    reqs.mod_hashes.insert(s.to_lowercase());
                }
                if let Some(s) = &m.sha512 {
                    reqs.mod_hashes.insert(s.to_lowercase());
                }
            }

            // Check version JSON for canonical asset index ID (only once per unique Minecraft version)
            if is_new_mc_version {
                let version_json = paths.version_json_path(&meta.manifest.minecraft_version);
                if let Ok(bytes) = std::fs::read(&version_json) {
                    if let Ok(manifest) = serde_json::from_slice::<crate::loaders::MojangVersionManifest>(&bytes) {
                        if let Some(a) = manifest.asset_index {
                            reqs.asset_index_ids.insert(a.id);
                        }
                    }
                }
                reqs.asset_index_ids.insert(meta.manifest.minecraft_version.clone());
            }

            loaded_manifest = true;
        }

        // 2차 안전망: 손상/누락된 인스턴스 메타데이터 대응 - 물리적 mods 파일 해시 수집
        if !loaded_manifest {
            let mods_dir = inst_dir.join("mods");
            if mods_dir.is_dir() {
                if let Ok(mod_files) = std::fs::read_dir(mods_dir) {
                    for mod_entry in mod_files.flatten() {
                        let p = mod_entry.path();
                        if p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("jar") {
                            #[cfg(unix)]
                            {
                                use std::os::unix::fs::MetadataExt;
                                if let Ok(meta) = mod_entry.metadata() {
                                    if let Some(hash) = store_inodes.get(&meta.ino()) {
                                        reqs.mod_hashes.insert(hash.clone());
                                        continue;
                                    }
                                }
                            }

                            if let Ok(sha1_hash) = crate::store::compute_file_hash(&p, crate::manifest::HashAlgorithm::Sha1) {
                                reqs.mod_hashes.insert(sha1_hash.to_lowercase());
                            }
                            if let Ok(sha512_hash) = crate::store::compute_file_hash(&p, crate::manifest::HashAlgorithm::Sha512) {
                                reqs.mod_hashes.insert(sha512_hash.to_lowercase());
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(reqs)
}

/// Collects all valid asset object hashes referenced by the active asset index files.
pub fn collect_active_asset_hashes(paths: &SzzPaths, active_index_ids: &HashSet<String>) -> HashSet<String> {
    let mut hashes = HashSet::new();
    let indexes_dir = paths.assets_indexes_dir();
    if !indexes_dir.exists() {
        return hashes;
    }

    // Also look for indexes that start with active index IDs (e.g. "1.20" for "1.20.1")
    if let Ok(entries) = std::fs::read_dir(&indexes_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("json") {
                let id = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let is_active = is_asset_index_active(id, active_index_ids);

                if is_active {
                    if let Ok(bytes) = std::fs::read(&p) {
                        if let Ok(index) = serde_json::from_slice::<crate::assets::AssetIndexFile>(&bytes) {
                            for (_, obj) in index.objects {
                                hashes.insert(obj.hash.to_lowercase());
                            }
                        }
                    }
                }
            }
        }
    }

    hashes
}

/// Analyzes storage usage across all categories and computes reclaimable byte amounts.
pub fn get_storage_usage(paths: &SzzPaths) -> Result<StorageUsageReport> {
    let reqs = collect_active_requirements(paths)?;
    let active_asset_hashes = collect_active_asset_hashes(paths, &reqs.asset_index_ids);

    let mut report = StorageUsageReport::default();

    // 1. CAS Mods (`~/.szz/store/`)
    let store_dir = paths.store_dir();
    if store_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&store_dir) {
            let now = std::time::SystemTime::now();
            for entry in entries.flatten() {
                let p = entry.path();
                if !p.is_file() {
                    continue;
                }
                let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
                let len = p.metadata().map(|m| m.len()).unwrap_or(0);
                report.mods.total_bytes = report.mods.total_bytes.saturating_add(len);
                report.mods.item_count += 1;

                if ext == "tmp" {
                    let is_old = p.metadata()
                        .and_then(|m| m.modified())
                        .map(|mod_time| now.duration_since(mod_time).map(|d| d.as_secs() > 3600).unwrap_or(false))
                        .unwrap_or(false);
                    if is_old {
                        report.mods.reclaimable_bytes = report.mods.reclaimable_bytes.saturating_add(len);
                        report.mods.reclaimable_count += 1;
                    }
                } else if ext == "jar" {
                    if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                        let hash = stem.to_lowercase();
                        if !reqs.mod_hashes.contains(&hash) {
                            if let Ok(nlink) = crate::store::file_hardlink_count(&p) {
                                if nlink <= 1 {
                                    report.mods.reclaimable_bytes = report.mods.reclaimable_bytes.saturating_add(len);
                                    report.mods.reclaimable_count += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Java Runtimes (`~/.szz/runtimes/`)
    let runtimes_dir = paths.runtimes_dir();
    if runtimes_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&runtimes_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if !p.is_dir() || p.is_symlink() {
                    continue;
                }
                let dir_name = entry.file_name().to_string_lossy().to_string();
                if let Some(ver) = parse_java_runtime_dir_version(&dir_name) {
                    let (size, _) = calculate_dir_size(&p);
                    report.runtimes.total_bytes = report.runtimes.total_bytes.saturating_add(size);
                    report.runtimes.item_count += 1;

                    if !reqs.java_versions.contains(&ver) {
                        report.runtimes.reclaimable_bytes = report.runtimes.reclaimable_bytes.saturating_add(size);
                        report.runtimes.reclaimable_count += 1;
                    }
                }
            }
        }
    }

    // 3. Mojang Assets (`~/.szz/assets/`)
    let safe_to_scan_asset_objects = reqs.instance_count == 0 || !active_asset_hashes.is_empty();
    let assets_objects_dir = paths.assets_objects_dir();
    if assets_objects_dir.exists() {
        if let Ok(subdirs) = std::fs::read_dir(&assets_objects_dir) {
            for subdir in subdirs.flatten() {
                if let Ok(file_type) = subdir.file_type() {
                    if !file_type.is_dir() {
                        continue;
                    }
                }
                if let Ok(obj_files) = std::fs::read_dir(subdir.path()) {
                    for obj in obj_files.flatten() {
                        if let Ok(file_type) = obj.file_type() {
                            if file_type.is_file() {
                                let len = obj.metadata().map(|m| m.len()).unwrap_or(0);
                                report.assets.total_bytes = report.assets.total_bytes.saturating_add(len);
                                report.assets.item_count += 1;

                                if safe_to_scan_asset_objects {
                                    let file_name = obj.file_name();
                                    if let Some(hash) = file_name.to_str() {
                                        let is_in_use = if hash.bytes().all(|b| !b.is_ascii_uppercase()) {
                                            active_asset_hashes.contains(hash)
                                        } else {
                                            active_asset_hashes.contains(&hash.to_lowercase())
                                        };
                                        if !is_in_use {
                                            report.assets.reclaimable_bytes = report.assets.reclaimable_bytes.saturating_add(len);
                                            report.assets.reclaimable_count += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    // Also add unused index files
    let indexes_dir = paths.assets_indexes_dir();
    if indexes_dir.exists() {
        if let Ok(index_files) = std::fs::read_dir(&indexes_dir) {
            for entry in index_files.flatten() {
                let p = entry.path();
                if p.is_file() {
                    let len = p.metadata().map(|m| m.len()).unwrap_or(0);
                    report.assets.total_bytes = report.assets.total_bytes.saturating_add(len);
                    let id = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    let is_active = is_asset_index_active(id, &reqs.asset_index_ids);
                    if !is_active {
                        report.assets.reclaimable_bytes = report.assets.reclaimable_bytes.saturating_add(len);
                    }
                }
            }
        }
    }

    // 4. Libraries (`~/.szz/libraries/`)
    let libraries_dir = paths.libraries_dir();
    if libraries_dir.exists() {
        let (lib_size, lib_count) = calculate_dir_size(&libraries_dir);
        report.libraries.total_bytes = lib_size;
        report.libraries.item_count = lib_count;

        // If no instances are installed at all, all libraries are reclaimable
        if reqs.instance_count == 0 {
            report.libraries.reclaimable_bytes = lib_size;
            report.libraries.reclaimable_count = lib_count;
        }
    }

    // 5. Instances (`~/.szz/instances/`)
    let instances_dir = paths.instances_dir();
    if instances_dir.exists() {
        let (inst_size, _) = calculate_dir_size(&instances_dir);
        report.instances.total_bytes = inst_size;
        report.instances.item_count = reqs.instance_count;
        report.instances.reclaimable_bytes = 0; // Instances are uninstalled via explicit UI/CLI commands
        report.instances.reclaimable_count = 0;
    }

    report.total_bytes = report.mods.total_bytes
        .saturating_add(report.runtimes.total_bytes)
        .saturating_add(report.assets.total_bytes)
        .saturating_add(report.libraries.total_bytes)
        .saturating_add(report.instances.total_bytes);

    report.reclaimable_bytes = report.mods.reclaimable_bytes
        .saturating_add(report.runtimes.reclaimable_bytes)
        .saturating_add(report.assets.reclaimable_bytes)
        .saturating_add(report.libraries.reclaimable_bytes);

    Ok(report)
}

/// Executes garbage collection to clean unused cache based on provided options.
pub fn clean_unused_cache(paths: &SzzPaths, options: &CleanOptions) -> Result<CleanSummary> {
    // 1. Acquire global GC lock
    let _lock = GcLockGuard::acquire()?;

    // 2. Process running guard: Reject if any instance is running
    crate::instance::check_no_instance_running(paths)?;

    let mut summary = CleanSummary {
        dry_run: options.dry_run,
        ..Default::default()
    };

    if options.dry_run {
        let report = get_storage_usage(paths)?;
        if options.clean_mods {
            summary.reclaimed_bytes = summary.reclaimed_bytes.saturating_add(report.mods.reclaimable_bytes);
            summary.deleted_mods_count = report.mods.reclaimable_count;
        }
        if options.clean_runtimes {
            summary.reclaimed_bytes = summary.reclaimed_bytes.saturating_add(report.runtimes.reclaimable_bytes);
            summary.deleted_runtimes_count = report.runtimes.reclaimable_count;
        }
        if options.clean_assets {
            summary.reclaimed_bytes = summary.reclaimed_bytes.saturating_add(report.assets.reclaimable_bytes);
            summary.deleted_assets_count = report.assets.reclaimable_count;
        }
        if options.clean_libraries {
            summary.reclaimed_bytes = summary.reclaimed_bytes.saturating_add(report.libraries.reclaimable_bytes);
            summary.deleted_libraries_count = report.libraries.reclaimable_count;
        }
        return Ok(summary);
    }

    let reqs = collect_active_requirements(paths)?;

    // 1. Clean CAS Mods
    if options.clean_mods {
        let prune_summary = crate::store::prune_orphaned_mods_with_hashes(paths, &reqs.mod_hashes)?;
        summary.reclaimed_bytes = summary.reclaimed_bytes.saturating_add(prune_summary.reclaimed_bytes);
        summary.deleted_mods_count = prune_summary.pruned_count;
    }

    // 2. Clean Java Runtimes
    if options.clean_runtimes {
        let runtimes_dir = paths.runtimes_dir();
        if runtimes_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&runtimes_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if !p.is_dir() || p.is_symlink() {
                        continue;
                    }
                    let dir_name = entry.file_name().to_string_lossy().to_string();
                    if let Some(ver) = parse_java_runtime_dir_version(&dir_name) {
                        // Strict validation: must be inside runtimes_dir
                        if p.parent() == Some(&runtimes_dir) && !reqs.java_versions.contains(&ver) {
                            let (size, _) = calculate_dir_size(&p);
                            if std::fs::remove_dir_all(&p).is_ok() {
                                summary.reclaimed_bytes = summary.reclaimed_bytes.saturating_add(size);
                                summary.deleted_runtimes_count += 1;
                                println!("[szz-core] Removed unused Java runtime: {}", dir_name);
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Clean Mojang Assets
    if options.clean_assets {
        let active_asset_hashes = collect_active_asset_hashes(paths, &reqs.asset_index_ids);
        let assets_objects_dir = paths.assets_objects_dir();
        if reqs.instance_count > 0 && active_asset_hashes.is_empty() {
            eprintln!("[szz-core] Warning: Skipping asset objects cleanup: active instances exist but 0 asset hashes resolved");
        } else if assets_objects_dir.exists() {
            if let Ok(subdirs) = std::fs::read_dir(&assets_objects_dir) {
                for subdir in subdirs.flatten() {
                    let sub_path = subdir.path();
                    if !sub_path.is_dir() {
                        continue;
                    }
                    if let Ok(obj_files) = std::fs::read_dir(&sub_path) {
                        for obj in obj_files.flatten() {
                            let obj_path = obj.path();
                            if obj_path.is_file() {
                                if let Some(hash) = obj_path.file_name().and_then(|s| s.to_str()) {
                                    let is_in_use = if hash.bytes().all(|b| !b.is_ascii_uppercase()) {
                                        active_asset_hashes.contains(hash)
                                    } else {
                                        active_asset_hashes.contains(&hash.to_lowercase())
                                    };
                                    if !is_in_use {
                                        let len = obj.metadata().map(|m| m.len()).unwrap_or(0);
                                        if std::fs::remove_file(&obj_path).is_ok() {
                                            summary.reclaimed_bytes = summary.reclaimed_bytes.saturating_add(len);
                                            summary.deleted_assets_count += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Clean unused asset indexes
        let indexes_dir = paths.assets_indexes_dir();
        if indexes_dir.exists() {
            if let Ok(index_files) = std::fs::read_dir(&indexes_dir) {
                for entry in index_files.flatten() {
                    let p = entry.path();
                    if p.is_file() {
                        let id = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                        let is_active = is_asset_index_active(id, &reqs.asset_index_ids);
                        if !is_active {
                            let len = p.metadata().map(|m| m.len()).unwrap_or(0);
                            if std::fs::remove_file(&p).is_ok() {
                                summary.reclaimed_bytes = summary.reclaimed_bytes.saturating_add(len);
                            }
                        }
                    }
                }
            }
        }
    }

    // 4. Clean Libraries
    if options.clean_libraries {
        let libraries_dir = paths.libraries_dir();
        if libraries_dir.exists() && reqs.instance_count == 0 {
            let (size, count) = calculate_dir_size(&libraries_dir);
            if std::fs::remove_dir_all(&libraries_dir).is_ok() {
                let _ = std::fs::create_dir_all(&libraries_dir);
                summary.reclaimed_bytes = summary.reclaimed_bytes.saturating_add(size);
                summary.deleted_libraries_count = count;
                println!("[szz-core] Cleaned all libraries (0 active instances)");
            }
        }
    }

    println!(
        "[szz-core] Storage GC finished: {} bytes reclaimed ({} runtimes, {} assets, {} mods)",
        summary.reclaimed_bytes,
        summary.deleted_runtimes_count,
        summary.deleted_assets_count,
        summary.deleted_mods_count
    );

    Ok(summary)
}
