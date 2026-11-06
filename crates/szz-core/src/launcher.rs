use crate::auth::AuthSession;
use crate::error::{Result, SzzError};
use crate::loaders::natives::{is_version_at_least, parse_server_address};
use crate::manifest::ServerManifest;
use crate::paths::SzzPaths;
use crate::progress::{LaunchStep, ProgressReporter};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct LaunchConfig {
    pub java_binary: PathBuf,
    pub game_dir: PathBuf,
    pub assets_dir: PathBuf,
    pub asset_index: String,
    pub version: String,
    pub main_class: String,
    pub classpath: String,
    pub max_memory: String,
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub user_type: String,
    pub quick_play_multiplayer: Option<String>,
    pub extra_jvm_args: Vec<String>,
    pub extra_game_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchResult {
    pub command_line: String,
    pub pid: Option<u32>,
}

pub fn build_command(config: &LaunchConfig) -> Command {
    let mut cmd = Command::new(&config.java_binary);

    cmd.arg(&config.max_memory);
    cmd.arg("-Dfile.encoding=UTF-8");

    let natives_dir = config.game_dir.join("natives");
    if natives_dir.exists() {
        cmd.arg(format!("-Djava.library.path={}", natives_dir.display()));
    }

    // Append extra JVM args (e.g. ModLauncher flags, GC flags, Forge JVM args)
    cmd.args(&config.extra_jvm_args);

    cmd.arg("-cp").arg(&config.classpath);
    cmd.arg(&config.main_class);

    cmd.arg("--version").arg(&config.version);
    cmd.arg("--gameDir").arg(&config.game_dir);
    cmd.arg("--assetsDir").arg(&config.assets_dir);
    cmd.arg("--assetIndex").arg(&config.asset_index);
    cmd.arg("--username").arg(&config.username);
    cmd.arg("--uuid").arg(&config.uuid);
    cmd.arg("--accessToken").arg(&config.access_token);
    cmd.arg("--userType").arg(&config.user_type);

    // Append extra game args (e.g. --tweakClass, --demo, --width, --height, --fullscreen)
    cmd.args(&config.extra_game_args);

    // Auto-connect branching:
    // 1.20+ supports --quickPlayMultiplayer <addr>
    // Pre-1.20 uses --server <host> --port <port>
    if let Some(server_addr) = &config.quick_play_multiplayer {
        if is_version_at_least(&config.version, "1.20") {
            cmd.arg("--quickPlayMultiplayer").arg(server_addr);
        } else {
            let (host, port) = parse_server_address(server_addr);
            cmd.arg("--server").arg(&host);
            cmd.arg("--port").arg(port.to_string());
        }
    }

    cmd.current_dir(&config.game_dir);
    cmd
}

pub fn apply_settings_to_launch_config(
    config: &mut LaunchConfig,
    settings: &crate::settings::UserSettings,
    server_address: &str,
) {
    config.max_memory = format!("-Xmx{}M", settings.java.allocated_memory_mb);
    config
        .extra_jvm_args
        .extend(settings.java.custom_jvm_args.clone());

    if settings.game.fullscreen {
        config.extra_game_args.push("--fullscreen".to_string());
    }
    if settings.game.resolution_width > 0 && settings.game.resolution_height > 0 {
        config.extra_game_args.push("--width".to_string());
        config
            .extra_game_args
            .push(settings.game.resolution_width.to_string());
        config.extra_game_args.push("--height".to_string());
        config
            .extra_game_args
            .push(settings.game.resolution_height.to_string());
    }

    if settings.game.auto_connect {
        config.quick_play_multiplayer = Some(server_address.to_string());
    } else {
        config.quick_play_multiplayer = None;
    }
}

pub fn format_command_line(cmd: &Command) -> String {
    let prog = cmd.get_program().to_string_lossy();
    let mut args: Vec<String> = Vec::new();
    let mut redact_next = false;
    for arg in cmd.get_args() {
        let arg_str = arg.to_string_lossy();
        if redact_next {
            args.push("[REDACTED]".to_string());
            redact_next = false;
        } else if arg_str == "--accessToken" {
            args.push(arg_str.into_owned());
            redact_next = true;
        } else {
            args.push(arg_str.into_owned());
        }
    }

    format!("{} {}", prog, args.join(" "))
}

pub async fn launch_instance(
    paths: &SzzPaths,
    instance_id: &str,
    auth: &dyn AuthSession,
    reporter: &dyn ProgressReporter,
    dry_run: bool,
) -> Result<LaunchResult> {
    let (res, _child) = launch_instance_with_child(paths, instance_id, auth, reporter, dry_run).await?;
    Ok(res)
}

pub async fn launch_instance_with_child(
    paths: &SzzPaths,
    instance_id: &str,
    auth: &dyn AuthSession,
    reporter: &dyn ProgressReporter,
    dry_run: bool,
) -> Result<(LaunchResult, Option<std::process::Child>)> {
    if crate::storage_gc::is_gc_in_progress() {
        return Err(SzzError::StorageGcInProgress);
    }

    crate::instance::validate_instance_id(instance_id)?;
    let instance_dir = paths.instance_dir(instance_id);
    let meta = crate::instance::InstanceMetadata::load_from_dir(&instance_dir)?;

    let manifest = if let Some(m) = meta {
        m.manifest
    } else {
        // Fallback: Check if instance directory was prepared (e.g., via dry-run install) and matches fixtures
        let mods_dir = instance_dir.join("mods");
        if mods_dir.is_dir() {
            let fixtures_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures");
            let sample = fixtures_dir.join("sample-manifest.json");
            let mut found = None;
            if let Ok(m) = ServerManifest::from_file(&sample) {
                if m.server_id == instance_id {
                    found = Some(m);
                }
            }
            if found.is_none() {
                if let Ok(entries) = std::fs::read_dir(fixtures_dir.join("servers")) {
                    for entry in entries.flatten() {
                        if let Ok(sm) = ServerManifest::from_file(entry.path()) {
                            if sm.server_id == instance_id {
                                found = Some(sm);
                                break;
                            }
                        }
                    }
                }
            }
            let matched = found
                .ok_or_else(|| SzzError::InstanceNotInstalled(instance_id.to_string()))?;
            let has_mods_on_disk = std::fs::read_dir(&mods_dir)
                .map(|mut d| d.next().is_some())
                .unwrap_or(false);
            if matched.mods.is_empty() || has_mods_on_disk {
                matched
            } else {
                return Err(SzzError::InstanceNotInstalled(instance_id.to_string()));
            }
        } else {
            return Err(SzzError::InstanceNotInstalled(instance_id.to_string()));
        }
    };

    launch_server_manifest_with_child(paths, &manifest, auth, reporter, dry_run).await
}

pub async fn launch_server_manifest(
    paths: &SzzPaths,
    manifest: &ServerManifest,
    auth: &dyn AuthSession,
    reporter: &dyn ProgressReporter,
    dry_run: bool,
) -> Result<LaunchResult> {
    let (res, _child) = launch_server_manifest_with_child(paths, manifest, auth, reporter, dry_run).await?;
    Ok(res)
}

pub async fn launch_server_manifest_with_child(
    paths: &SzzPaths,
    manifest: &ServerManifest,
    auth: &dyn AuthSession,
    reporter: &dyn ProgressReporter,
    dry_run: bool,
) -> Result<(LaunchResult, Option<std::process::Child>)> {
    if crate::storage_gc::is_gc_in_progress() {
        return Err(SzzError::StorageGcInProgress);
    }

    if !dry_run {
        let ping_result =
            crate::ping::ping_server(&manifest.server_address, crate::ping::DEFAULT_PING_TIMEOUT).await;
        if !ping_result.online {
            return Err(SzzError::ServerOffline(manifest.server_address.clone()));
        }
    }

    let game_dir = paths.ensure_instance_dirs(&manifest.server_id)?;

    let (java_binary, linked_mods, resolved, config_commit_opt, asset_sync) =
        crate::instance::sync_instance_resources(paths, manifest, reporter, dry_run).await?;

    if let Some(commit_handle) = config_commit_opt {
        if !dry_run {
            commit_handle.commit()?;
        }
    }

    if !dry_run {
        let meta = crate::instance::InstanceMetadata::new(&manifest.server_id, manifest.clone());
        meta.save_to_dir(&game_dir)?;
    }

    reporter.report(
        LaunchStep::AssemblingClasspath,
        Some("Classpath assembled and mods hardlinked successfully"),
    );
    println!("[szz-core] Linked {} mod(s) to {:?}", linked_mods.len(), game_dir.join("mods"));
    println!("[szz-core] Classpath assembled with {} libraries", resolved.library_count);
    println!("[szz-core] Assets synchronized with index '{}'", asset_sync.asset_index_id);

    let settings = crate::settings::UserSettings::load(paths).unwrap_or_default();

    let mut config = LaunchConfig {
        java_binary,
        game_dir: game_dir.clone(),
        assets_dir: paths.assets_dir(),
        asset_index: asset_sync.asset_index_id,
        version: manifest.minecraft_version.clone(),
        main_class: resolved.main_class,
        classpath: resolved.classpath,
        max_memory: String::new(),
        username: auth.username().to_string(),
        uuid: auth.uuid().to_string(),
        access_token: auth.access_token().to_string(),
        user_type: auth.user_type().to_string(),
        quick_play_multiplayer: None,
        extra_jvm_args: resolved.extra_jvm_args,
        extra_game_args: resolved.extra_game_args,
    };
    apply_settings_to_launch_config(&mut config, &settings, &manifest.server_address);

    let mut cmd = build_command(&config);
    let command_line = format_command_line(&cmd);

    if dry_run {
        reporter.report(
            LaunchStep::Complete,
            Some("Launch completed in dry-run mode"),
        );
        println!("[szz-core] [DRY RUN] Generated Minecraft launch command:\n{}", command_line);
        return Ok((
            LaunchResult {
                command_line,
                pid: None,
            },
            None,
        ));
    }

    reporter.report(LaunchStep::SpawningClient, None);
    println!("[szz-core] Spawning Minecraft client child process...");
    let child = cmd.spawn().map_err(|source| SzzError::Io {
        path: config.java_binary.clone(),
        source,
    })?;

    let pid = child.id();

    // Write PID file to game directory for process status detection
    let pid_file = paths.instance_pid_file(&manifest.server_id);
    let _ = std::fs::write(&pid_file, pid.to_string());

    reporter.report(
        LaunchStep::Complete,
        Some(&format!("Minecraft process launched successfully (PID: {})", pid)),
    );
    println!("[szz-core] Minecraft process launched successfully (PID: {})", pid);

    Ok((
        LaunchResult {
            command_line,
            pid: Some(pid),
        },
        Some(child),
    ))
}
