pub mod assets;
pub mod auth;
pub mod catalog;
pub mod config;
pub mod config_sync;
pub mod error;
pub mod fabric;
pub mod instance;
pub mod launcher;
pub mod loaders;
pub mod manifest;
pub mod paths;
pub mod ping;
pub mod progress;
pub mod runtime;
pub mod settings;
pub mod storage_gc;
pub mod store;

pub use assets::{sync_mojang_assets, validate_asset_index_id, validate_hash, AssetIndexFile, AssetObject, AssetSyncResult};
pub use auth::{
    AccountProfile, AuthSession, AuthStatePayload, DevAuthSession, MicrosoftAuthSession,
};
pub use catalog::{
    fetch_remote_catalog, fetch_server_manifest, load_cached_catalog, load_cache_meta,
    save_cached_catalog, validate_url_scheme, CatalogCacheMeta, CatalogFetchOutput,
    RemoteCatalog, RemoteCatalogServerEntry, CATALOG_NETWORK_TIMEOUT,
};
pub use config::{
    catalog_url, find_dotenv_file, load_dotenv, msa_client_id, parse_dotenv_entries,
    DEFAULT_CATALOG_URL,
};
pub use config_sync::{
    apply_config_archive, create_overrides_backup, is_dangerous_executable, is_protected_user_path,
    is_sensitive_system_path, is_user_customizable_config, prune_stale_managed_files,
    sync_server_config, verify_bytes_hash, ConfigCommitHandle, ConfigState,
};
pub use error::{Result, SzzError};
pub use fabric::{
    fetch_fabric_metadata, maven_coordinate_to_path, sync_fabric_and_assemble_classpath,
    ResolvedClasspath,
};
pub use instance::{
    check_instance_update, check_no_instance_running, get_instance_status,
    get_instance_status_with_remote, install_instance, list_installed_instances,
    uninstall_instance, update_instance, validate_instance_id, InstanceLifecycleStatus,
    InstanceMetadata, InstanceUpdateDiff, InstalledInstanceInfo,
};
pub use launcher::{
    apply_settings_to_launch_config, build_command, format_command_line, launch_instance,
    launch_instance_with_child, launch_server_manifest, launch_server_manifest_with_child,
    LaunchConfig, LaunchResult,
};
pub use manifest::{ConfigSpec, HashAlgorithm, LoaderConfig, LoaderType, ModEntry, ServerManifest};
pub use paths::SzzPaths;
pub use ping::{
    build_slp_handshake_and_request, encode_varint, ping_server, read_varint,
    strip_minecraft_formatting, ServerStatusPayload, DEFAULT_PING_TIMEOUT,
};
pub use progress::{CliProgressReporter, LaunchStep, NoopProgressReporter, ProgressPayload, ProgressReporter};
pub use runtime::ensure_java_runtime;
pub use settings::{
    get_system_memory_info, GameSettings, JavaSettings, LauncherBehaviorOnLaunch, LauncherSettings,
    SystemMemoryInfo, UserSettings, DEFAULT_MEMORY_MB, DEFAULT_RESOLUTION_HEIGHT,
    DEFAULT_RESOLUTION_WIDTH, MIN_MEMORY_MB,
};
pub use storage_gc::{
    clean_unused_cache, get_storage_usage, is_gc_in_progress, CleanOptions, CleanSummary,
    GcLockGuard, StorageCategoryUsage, StorageUsageReport,
};
pub use store::{
    clean_stale_temp_files, compute_file_hash, create_hardlink_or_copy, ensure_mod,
    file_hardlink_count, prune_orphaned_mods, prune_orphaned_mods_with_hashes, sync_mods,
    sync_mods_with_reporter, verify_file_hash, PruneSummary, DEFAULT_MOD_CONCURRENCY,
};
