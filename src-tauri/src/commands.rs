use szz_core::auth::{
    AccountProfile, AuthStatePayload, DeviceCodeResponse,
};
use szz_core::instance::{
    InstanceLifecycleStatus, InstalledInstanceInfo,
};
use szz_core::launcher::LaunchResult;
use szz_core::manifest::ServerManifest;
use szz_core::paths::SzzPaths;
use szz_core::ping::ServerStatusPayload;
use szz_core::progress::{LaunchStep, ProgressPayload, ProgressReporter};
use szz_core::settings::{LauncherBehaviorOnLaunch, SystemMemoryInfo, UserSettings};
use tauri::{AppHandle, Emitter, Manager};

pub fn open_browser(url: &str) -> std::result::Result<(), String> {
    szz_core::auth::service::open_browser(url).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_external_url(url: String) -> std::result::Result<(), String> {
    open_browser(&url)
}

pub struct TauriProgressReporter {
    app: AppHandle,
}

impl TauriProgressReporter {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl ProgressReporter for TauriProgressReporter {
    fn report(&self, step: LaunchStep, custom_message: Option<&str>) {
        let payload = ProgressPayload::from_step(step, custom_message);
        let _ = self.app.emit("launch-progress", payload);
    }
}

#[tauri::command]
pub fn get_auth_state() -> std::result::Result<AuthStatePayload, String> {
    let paths = SzzPaths::new().map_err(|e| e.to_string())?;
    szz_core::auth::service::get_auth_state(&paths).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_login_loopback() -> std::result::Result<AccountProfile, String> {
    let paths = SzzPaths::new().map_err(|e| e.to_string())?;
    szz_core::auth::service::login_with_loopback(&paths, |url| {
        szz_core::auth::service::open_browser(url)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_login_device_code() -> std::result::Result<DeviceCodeResponse, String> {
    szz_core::auth::service::request_device_code()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn poll_login_device_code(
    device_code: String,
) -> std::result::Result<Option<AccountProfile>, String> {
    let paths = SzzPaths::new().map_err(|e| e.to_string())?;
    szz_core::auth::service::poll_and_complete_device_code(&paths, &device_code)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn switch_account(account_id: String) -> std::result::Result<(), String> {
    let paths = SzzPaths::new().map_err(|e| e.to_string())?;
    szz_core::auth::service::switch_active_account(&paths, &account_id)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_account(account_id: String) -> std::result::Result<(), String> {
    let paths = SzzPaths::new().map_err(|e| e.to_string())?;
    szz_core::auth::service::remove_account(&paths, &account_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_dev_account(username: String) -> std::result::Result<AccountProfile, String> {
    let paths = SzzPaths::new().map_err(|e| e.to_string())?;
    szz_core::auth::service::create_dev_account(&paths, &username).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_instance_status(
    instance_id: String,
) -> std::result::Result<InstanceLifecycleStatus, String> {
    let paths = SzzPaths::new().map_err(|e| e.to_string())?;
    szz_core::instance::get_instance_status(&paths, &instance_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn install_instance(
    app: AppHandle,
    manifest_path: String,
    instance_id: Option<String>,
    dry_run: bool,
) -> std::result::Result<(), String> {
    let mut manifest = tokio::task::spawn_blocking(move || ServerManifest::from_file(&manifest_path))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    if let Some(ref id) = instance_id {
        szz_core::validate_instance_id(id).map_err(|e| e.to_string())?;
        manifest.server_id = id.clone();
    }

    let paths = SzzPaths::new().map_err(|e| e.to_string())?;
    let reporter = TauriProgressReporter::new(app);

    szz_core::instance::install_instance(&paths, &manifest, &reporter, dry_run)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn uninstall_instance(instance_id: String) -> std::result::Result<(), String> {
    let paths = SzzPaths::new().map_err(|e| e.to_string())?;
    szz_core::instance::uninstall_instance(&paths, &instance_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_installed_instances() -> std::result::Result<Vec<InstalledInstanceInfo>, String> {
    let paths = SzzPaths::new().map_err(|e| e.to_string())?;
    szz_core::instance::list_installed_instances(&paths).map_err(|e| e.to_string())
}

fn apply_post_launch_window_behavior(app: &AppHandle, paths: &SzzPaths) {
    let settings = UserSettings::load(paths).unwrap_or_default();
    match settings.launcher.behavior_on_launch {
        LauncherBehaviorOnLaunch::KeepOpen => {}
        LauncherBehaviorOnLaunch::Minimize => {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.minimize();
            }
        }
        LauncherBehaviorOnLaunch::Close => {
            app.exit(0);
        }
    }
}

fn spawn_instance_exit_watcher(
    app: AppHandle,
    paths: SzzPaths,
    instance_id: String,
    mut child: std::process::Child,
) {
    std::thread::spawn(move || {
        let exit_status = child.wait();
        println!(
            "[szz-desktop] Instance '{}' process exited: {:?}",
            instance_id, exit_status
        );

        let pid_file = paths.instance_pid_file(&instance_id);
        let _ = std::fs::remove_file(pid_file);

        let settings = UserSettings::load(&paths).unwrap_or_default();
        if settings.launcher.behavior_on_launch == LauncherBehaviorOnLaunch::Minimize {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.unminimize();
                let _ = win.set_focus();
            }
        }

        let payload = serde_json::json!({
            "instanceId": instance_id,
            "status": "Installed"
        });

        let emit_res = app.emit("instance-status-changed", &payload);
        println!(
            "[szz-desktop] Emitted instance-status-changed for '{}': {:?}",
            instance_id, emit_res
        );
        let _ = app.emit_to("main", "instance-status-changed", &payload);
    });
}

#[tauri::command]
pub async fn launch_instance(
    app: AppHandle,
    instance_id: String,
    dry_run: bool,
) -> std::result::Result<LaunchResult, String> {
    let paths = SzzPaths::new().map_err(|e| e.to_string())?;
    let auth_session = szz_core::auth::service::ensure_active_session(&paths)
        .await
        .map_err(|e| e.to_string())?;

    let reporter = TauriProgressReporter::new(app.clone());
    let (result, child_opt) = szz_core::launcher::launch_instance_with_child(
        &paths,
        &instance_id,
        auth_session.as_ref(),
        &reporter,
        dry_run,
    )
    .await
    .map_err(|e| e.to_string())?;

    if let Some(child) = child_opt {
        apply_post_launch_window_behavior(&app, &paths);
        spawn_instance_exit_watcher(app, paths, instance_id, child);
    }

    Ok(result)
}

#[tauri::command]
pub async fn launch_server(
    app: AppHandle,
    manifest_path: String,
    dry_run: bool,
) -> std::result::Result<LaunchResult, String> {
    let manifest = tokio::task::spawn_blocking(move || ServerManifest::from_file(&manifest_path))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    let paths = SzzPaths::new().map_err(|e| e.to_string())?;
    let auth_session = szz_core::auth::service::ensure_active_session(&paths)
        .await
        .map_err(|e| e.to_string())?;

    let reporter = TauriProgressReporter::new(app.clone());
    let instance_id = manifest.server_id.clone();
    let (result, child_opt) = szz_core::launcher::launch_server_manifest_with_child(
        &paths,
        &manifest,
        auth_session.as_ref(),
        &reporter,
        dry_run,
    )
    .await
    .map_err(|e| e.to_string())?;

    if let Some(child) = child_opt {
        apply_post_launch_window_behavior(&app, &paths);
        spawn_instance_exit_watcher(app, paths, instance_id, child);
    }

    Ok(result)
}

#[tauri::command]
pub fn get_sample_manifest_path() -> String {
    let fixture_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("fixtures")
        .join("sample-manifest.json");
    fixture_path.to_string_lossy().to_string()
}

#[tauri::command]
pub fn load_manifest(manifest_path: String) -> std::result::Result<ServerManifest, String> {
    ServerManifest::from_file(&manifest_path).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCatalogItem {
    pub id: String,
    pub name: String,
    pub manifest_path: String,
    pub manifest: Option<ServerManifest>,
    pub tagline: String,
    pub status: Option<InstanceLifecycleStatus>,
    pub latest_manifest_version: Option<u32>,
    #[serde(default)]
    pub is_offline_cache: bool,
}

#[tauri::command]
pub async fn get_server_catalog() -> std::result::Result<Vec<ServerCatalogItem>, String> {
    let paths = SzzPaths::new().map_err(|e| e.to_string())?;
    let catalog_url = szz_core::config::catalog_url();

    // 1. Try fetching remote catalog
    if let Ok(result) = szz_core::catalog::fetch_catalog_default(&paths, &catalog_url).await {
        let mut items = Vec::new();
        let is_offline = result.is_offline;

        for entry in result.catalog.servers {
            // Guard against path traversal from untrusted remote server IDs
            if szz_core::validate_instance_id(&entry.id).is_err() {
                continue;
            }

            let status = szz_core::instance::get_instance_status_with_remote(
                &paths,
                &entry.id,
                Some(entry.latest_manifest_version),
            )
            .ok();

            let manifest_cache_file = paths.cache_dir().join(format!("manifest_{}.json", entry.id));
            let mut manifest = ServerManifest::from_file(&manifest_cache_file).ok();
            if let Some(ref mut m) = manifest {
                m.server_id = entry.id.clone();
            }

            // Only attempt network manifest download if not already cached at target version and online
            let needs_network_fetch = !is_offline
                && manifest
                    .as_ref()
                    .map(|m| m.manifest_version < entry.latest_manifest_version)
                    .unwrap_or(true);

            if needs_network_fetch {
                if let Ok(mut m) = szz_core::catalog::fetch_server_manifest_default(&entry.manifest_url).await {
                    m.server_id = entry.id.clone();
                    if let Ok(json) = serde_json::to_string_pretty(&m) {
                        let temp_manifest_cache = paths.cache_dir().join(format!("manifest_{}.json.tmp", entry.id));
                        if std::fs::write(&temp_manifest_cache, json).is_ok() {
                            let _ = std::fs::rename(&temp_manifest_cache, &manifest_cache_file);
                        }
                    }
                    manifest = Some(m);
                }
            }

            // Fallback 2: installed instance manifest (only if no update is pending)
            if manifest.is_none() && status != Some(InstanceLifecycleStatus::UpdateAvailable) {
                if let Ok(Some(meta)) = szz_core::instance::InstanceMetadata::load_from_dir(&paths.instance_dir(&entry.id)) {
                    let mut inst_manifest = meta.manifest;
                    inst_manifest.server_id = entry.id.clone();
                    // Cache the manifest standalone so manifest_path always points to a valid ServerManifest
                    if let Ok(json) = serde_json::to_string_pretty(&inst_manifest) {
                        let _ = std::fs::write(&manifest_cache_file, json);
                    }
                    manifest = Some(inst_manifest);
                }
            }

            let manifest_path = manifest_cache_file.to_string_lossy().to_string();

            items.push(ServerCatalogItem {
                id: entry.id,
                name: entry.name,
                manifest_path,
                manifest,
                tagline: entry.tagline,
                status,
                latest_manifest_version: Some(entry.latest_manifest_version),
                is_offline_cache: is_offline,
            });
        }

        if !items.is_empty() {
            return Ok(items);
        }
    }

    // Fallback: local fixtures
    let fixtures_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("fixtures");

    let mut catalog = Vec::new();

    // 1. sample-manifest.json
    let sample_path = fixtures_dir.join("sample-manifest.json");
    if let Ok(manifest) = ServerManifest::from_file(&sample_path) {
        let status = szz_core::instance::get_instance_status_with_remote(
            &paths,
            &manifest.server_id,
            Some(manifest.manifest_version),
        )
        .ok();
        catalog.push(ServerCatalogItem {
            id: manifest.server_id.clone(),
            name: manifest.name.clone(),
            manifest_path: sample_path.to_string_lossy().to_string(),
            manifest: Some(manifest.clone()),
            tagline: "Official SZZ test server featuring Fabric 1.20.1 and Content-Addressable Storage hardlinking.".to_string(),
            status,
            latest_manifest_version: Some(manifest.manifest_version),
            is_offline_cache: true,
        });
    }

    // 2. fixtures/servers/*.json
    let servers_dir = fixtures_dir.join("servers");
    if let Ok(entries) = std::fs::read_dir(servers_dir) {
        let mut server_files = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                server_files.push(path);
            }
        }
        server_files.sort();

        for path in server_files {
            if let Ok(manifest) = ServerManifest::from_file(&path) {
                let status = szz_core::instance::get_instance_status_with_remote(
                    &paths,
                    &manifest.server_id,
                    Some(manifest.manifest_version),
                )
                .ok();
                let tagline = match manifest.server_id.as_str() {
                    "cobblemon-1-21-1" => "Gotta Catch 'Em All in Minecraft! Full Fabric 1.21.1 Cobblemon adventure server.",
                    "hypixel-26-2" => "Modern client optimized with Sodium for Hypixel multiplayer.",
                    "hypixel-1-8-9" => "Classic Minecraft 1.8.9 pure vanilla client for Hypixel PvP.",
                    "skyfactory-4" | "skyfactory4-1-12-2" => "SkyFactory 4 Legacy Forge 1.12.2 full automation modpack experience.",
                    _ => "Modded Minecraft multiplayer server powered by szz.",
                }.to_string();

                catalog.push(ServerCatalogItem {
                    id: manifest.server_id.clone(),
                    name: manifest.name.clone(),
                    manifest_path: path.to_string_lossy().to_string(),
                    manifest: Some(manifest.clone()),
                    tagline,
                    status,
                    latest_manifest_version: Some(manifest.manifest_version),
                    is_offline_cache: true,
                });
            }
        }
    }

    Ok(catalog)
}

#[tauri::command]
pub async fn update_instance(
    app: AppHandle,
    mut manifest: ServerManifest,
    instance_id: Option<String>,
) -> std::result::Result<(), String> {
    if let Some(ref id) = instance_id {
        szz_core::validate_instance_id(id).map_err(|e| e.to_string())?;
        manifest.server_id = id.clone();
    }
    manifest.validate().map_err(|e| e.to_string())?;
    let paths = SzzPaths::new().map_err(|e| e.to_string())?;
    let reporter = TauriProgressReporter::new(app.clone());

    szz_core::instance::update_instance(&paths, &manifest, &reporter)
        .await
        .map_err(|e| e.to_string())?;

    let payload = serde_json::json!({
        "instanceId": manifest.server_id,
        "status": "Installed"
    });
    let _ = app.emit("instance-status-changed", &payload);

    Ok(())
}

#[tauri::command]
pub async fn get_storage_usage() -> std::result::Result<szz_core::storage_gc::StorageUsageReport, String> {
    let paths = SzzPaths::new().map_err(|e| e.to_string())?;
    tokio::task::spawn_blocking(move || {
        szz_core::storage_gc::get_storage_usage(&paths)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clean_storage_cache(
    options: szz_core::storage_gc::CleanOptions,
) -> std::result::Result<szz_core::storage_gc::CleanSummary, String> {
    let paths = SzzPaths::new().map_err(|e| e.to_string())?;
    tokio::task::spawn_blocking(move || {
        szz_core::storage_gc::clean_unused_cache(&paths, &options)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_settings() -> std::result::Result<UserSettings, String> {
    let paths = SzzPaths::new().map_err(|e| e.to_string())?;
    tokio::task::spawn_blocking(move || UserSettings::load(&paths))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_settings(settings: UserSettings) -> std::result::Result<UserSettings, String> {
    let paths = SzzPaths::new().map_err(|e| e.to_string())?;
    tokio::task::spawn_blocking(move || settings.save(&paths))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reset_settings() -> std::result::Result<UserSettings, String> {
    let paths = SzzPaths::new().map_err(|e| e.to_string())?;
    tokio::task::spawn_blocking(move || UserSettings::reset(&paths))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_system_memory_info() -> SystemMemoryInfo {
    szz_core::settings::get_system_memory_info()
}

#[tauri::command]
pub async fn ping_server(address: String) -> ServerStatusPayload {
    szz_core::ping::ping_server(&address, szz_core::ping::DEFAULT_PING_TIMEOUT).await
}
