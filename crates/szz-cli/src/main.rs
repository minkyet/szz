use clap::{Parser, Subcommand};
use std::path::PathBuf;
use szz_core::error::SzzError;
use szz_core::instance::{
    install_instance, list_installed_instances, uninstall_instance,
};
use szz_core::launcher::{launch_instance, launch_server_manifest};
use szz_core::manifest::ServerManifest;
use szz_core::paths::SzzPaths;
use szz_core::progress::CliProgressReporter;

#[derive(Parser)]
#[command(name = "szz-cli")]
#[command(about = "SZZ Minecraft mod launcher CLI harness", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Microsoft and offline developer account management
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },
    /// Inspect and validate a server manifest
    Inspect {
        /// Path to the manifest JSON file
        manifest_path: PathBuf,
    },
    /// Pre-install a server instance from a manifest without running the game
    Install {
        /// Path to the manifest JSON file
        manifest_path: PathBuf,
        /// Validate and plan installation without downloading artifacts
        #[arg(long)]
        dry_run: bool,
    },
    /// List all installed server instances
    #[command(alias = "ls")]
    List,
    /// Launch an installed server instance by its instance ID
    Run {
        /// Installed instance ID
        instance_id: String,
        /// Print the assembled launch command without executing the game process
        #[arg(long)]
        dry_run: bool,
    },
    /// Incrementally update an installed server instance
    Update {
        /// Target server instance ID to update
        instance_id: String,
        /// Optional path to new target manifest JSON file (if omitted, fetched from remote catalog)
        #[arg(long)]
        manifest: Option<PathBuf>,
    },
    /// Query the remote server catalog and list available servers
    Catalog {
        /// Optional custom catalog endpoint URL (defaults to SZZ_CATALOG_URL or production R2 catalog)
        #[arg(long)]
        url: Option<String>,
    },
    /// Uninstall and delete an installed server instance
    Uninstall {
        /// Installed instance ID to remove
        instance_id: String,
    },
    /// Manage global launcher settings and storage
    Settings {
        #[command(subcommand)]
        command: SettingsCommands,
    },
    /// Ping a Minecraft multiplayer server using the SLP protocol
    Ping {
        /// Target server address (e.g. play.example.com:25565)
        server_address: String,
    },
    /// Manage global storage, inspect disk usage, and clean unused cache (alias for `settings storage`)
    Storage {
        #[command(subcommand)]
        command: StorageCommands,
    },
}

#[derive(Subcommand)]
enum SettingsCommands {
    /// Inspect current launcher settings (or a specific setting key)
    #[command(alias = "get", alias = "info")]
    Status {
        /// Optional setting key (e.g. java.allocated_memory_mb, game.auto_connect)
        key: Option<String>,
    },
    /// Update a specific launcher setting key
    Set {
        /// Setting key (e.g. java.allocated_memory_mb, game.fullscreen)
        key: String,
        /// New value
        value: String,
    },
    /// Reset all launcher settings back to defaults
    Reset,
    /// Manage global storage, inspect disk usage, and clean unused cache
    Storage {
        #[command(subcommand)]
        command: StorageCommands,
    },
}

#[derive(Subcommand)]
enum StorageCommands {
    /// Inspect storage usage across all categories
    #[command(alias = "info")]
    Status,
    /// Clean unused and orphaned cache resources
    Clean {
        /// Calculate and display what would be cleaned without deleting files
        #[arg(long)]
        dry_run: bool,
        /// Only clean orphaned CAS mods
        #[arg(long)]
        only_mods: bool,
        /// Only clean unused Java runtimes
        #[arg(long)]
        only_runtimes: bool,
        /// Only clean unused Mojang assets
        #[arg(long)]
        only_assets: bool,
        /// Only clean unused libraries
        #[arg(long)]
        only_libraries: bool,
    },
}

#[derive(Subcommand)]
enum AuthCommands {
    /// Log in to a Microsoft account
    Login {
        /// Use Microsoft OAuth device code flow instead of local browser redirect
        #[arg(long)]
        device_code: bool,
    },
    /// List all configured accounts
    #[command(alias = "ls")]
    List,
    /// Switch active account by username or account ID
    Switch {
        /// Username or account ID to activate
        identifier: String,
    },
    /// Remove an account by username or account ID
    Remove {
        /// Username or account ID to remove
        identifier: String,
    },
    /// Create and activate an offline development account
    AddDev {
        /// Offline username
        username: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let paths = SzzPaths::new()?;
    let reporter = CliProgressReporter;

    match cli.command {
        Commands::Auth { command } => match command {
            AuthCommands::Login { device_code } => {
                if device_code {
                    let resp = szz_core::auth::service::request_device_code().await?;
                    println!("==================================================");
                    println!("To sign in with Microsoft, open the following page in your web browser:");
                    println!("  {}", resp.verification_uri);
                    println!("and enter the code:");
                    println!("  {}", resp.user_code);
                    println!("==================================================");
                    println!("Waiting for authorization to complete (press Ctrl+C to cancel)...");

                    let interval = std::time::Duration::from_secs(resp.interval.max(2));
                    let deadline = std::time::Instant::now()
                        + std::time::Duration::from_secs(resp.expires_in);
                    loop {
                        if std::time::Instant::now() >= deadline {
                            eprintln!(
                                "Error: Device code has expired ({}s). Please run 'szz-cli auth login --device-code' again.",
                                resp.expires_in
                            );
                            std::process::exit(1);
                        }
                        tokio::time::sleep(interval).await;
                        match szz_core::auth::service::poll_and_complete_device_code(
                            &paths,
                            &resp.device_code,
                        )
                        .await
                        {
                            Ok(Some(profile)) => {
                                println!(
                                    "Successfully logged in as '{}' ({})!",
                                    profile.username, profile.id
                                );
                                break;
                            }
                            Ok(None) => {}
                            Err(e) => {
                                eprintln!("Authentication failed: {}", e);
                                std::process::exit(1);
                            }
                        }
                    }
                } else {
                    println!("Starting local authentication listener...");
                    match szz_core::auth::service::login_with_loopback(&paths, |url| {
                        println!("Opening browser for Microsoft authentication:");
                        println!("  {}", url);
                        szz_core::auth::service::open_browser(url)
                    })
                    .await
                    {
                        Ok(profile) => {
                            println!(
                                "Successfully logged in as '{}' ({})!",
                                profile.username, profile.id
                            );
                        }
                        Err(e) => {
                            eprintln!("Login failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
            }
            AuthCommands::List => {
                let state = szz_core::auth::service::get_auth_state(&paths)?;
                if state.accounts.is_empty() {
                    println!("No accounts configured.");
                    println!("Run 'szz-cli auth login' or 'szz-cli auth add-dev <username>' to add an account.");
                } else {
                    println!(
                        "{:<3} {:<10} {:<24} {:<36}",
                        "ACT", "TYPE", "USERNAME", "ACCOUNT ID"
                    );
                    println!("{}", "-".repeat(78));
                    for acc in state.accounts {
                        let act = if acc.is_active { "*" } else { " " };
                        let acc_type = format!("{:?}", acc.account_type);
                        println!(
                            "{:<3} {:<10} {:<24} {:<36}",
                            act, acc_type, acc.username, acc.id
                        );
                    }
                }
            }
            AuthCommands::Switch { identifier } => {
                match szz_core::auth::service::switch_active_account(&paths, &identifier) {
                    Ok(profile) => {
                        println!(
                            "Switched active account to '{}' ({}).",
                            profile.username, profile.id
                        );
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            AuthCommands::Remove { identifier } => {
                match szz_core::auth::service::remove_account(&paths, &identifier) {
                    Ok(_) => {
                        println!("Account '{}' removed successfully.", identifier);
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            AuthCommands::AddDev { username } => {
                match szz_core::auth::service::create_dev_account(&paths, &username) {
                    Ok(profile) => {
                        println!(
                            "Created and activated development account '{}' ({}).",
                            profile.username, profile.id
                        );
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        },
        Commands::Inspect { manifest_path } => {
            println!("Reading manifest from {:?}...", manifest_path);
            let manifest = ServerManifest::from_file(&manifest_path)?;
            println!("Server ID: {}", manifest.server_id);
            println!("Server Name: {}", manifest.name);
            println!("Target Server: {}", manifest.server_address);
            println!("Minecraft Version: {}", manifest.minecraft_version);
            println!(
                "Loader: {:?} (v{})",
                manifest.loader.loader_type, manifest.loader.version
            );
            println!(
                "Required Java Version: Java {}",
                manifest.required_java_version()
            );
            println!(
                "Verified Sources: {}",
                if manifest.is_verified() {
                    "Yes (Official Modrinth/CurseForge)"
                } else {
                    "No (Contains Custom/External Mods)"
                }
            );
            println!("Total Mods: {}", manifest.mods.len());
            for (idx, m) in manifest.mods.iter().enumerate() {
                println!("  [{}] {} ({})", idx + 1, m.name, m.source);
            }
        }
        Commands::Install {
            manifest_path,
            dry_run,
        } => {
            println!("Reading manifest from {:?}...", manifest_path);
            let manifest = ServerManifest::from_file(&manifest_path)?;
            println!(
                "Installing instance '{}' ({}) (dry_run: {})...",
                manifest.name, manifest.server_id, dry_run
            );
            install_instance(&paths, &manifest, &reporter, dry_run).await?;
            if dry_run {
                println!("Instance installation planned successfully (dry-run).");
            } else {
                println!(
                    "Instance '{}' installed successfully! You can run it with: szz-cli run {}",
                    manifest.server_id, manifest.server_id
                );
            }
        }
        Commands::List => {
            let instances = list_installed_instances(&paths)?;
            if instances.is_empty() {
                println!("No instances installed.");
                println!("Run 'szz-cli install <manifest_path>' to install an instance.");
            } else {
                println!(
                    "{:<20} {:<30} {:<12} {:<16} {:<6} {:<10}",
                    "INSTANCE ID", "SERVER NAME", "MINECRAFT", "LOADER", "MODS", "STATUS"
                );
                println!("{}", "-".repeat(98));
                for inst in instances {
                    println!(
                        "{:<20} {:<30} {:<12} {:<16} {:<6} {:?}",
                        inst.instance_id,
                        inst.server_name,
                        inst.minecraft_version,
                        inst.loader,
                        inst.mod_count,
                        inst.status
                    );
                }
            }
        }
        Commands::Run {
            instance_id,
            dry_run,
        } => {
            let auth_session = match szz_core::auth::service::ensure_active_session(&paths).await {
                Ok(session) => session,
                Err(e) => {
                    eprintln!("Authentication error: {}", e);
                    std::process::exit(1);
                }
            };

            let maybe_manifest_path = std::path::Path::new(&instance_id);
            let launch_res = if maybe_manifest_path.exists()
                && maybe_manifest_path.extension().is_some_and(|e| e == "json")
            {
                let manifest = ServerManifest::from_file(maybe_manifest_path)?;
                println!(
                    "Launching manifest '{}' ({}) with user '{}'...",
                    manifest.name,
                    manifest.server_id,
                    auth_session.username()
                );
                launch_server_manifest(
                    &paths,
                    &manifest,
                    auth_session.as_ref(),
                    &reporter,
                    dry_run,
                )
                .await
            } else {
                println!(
                    "Launching instance '{}' with user '{}'...",
                    instance_id,
                    auth_session.username()
                );
                launch_instance(
                    &paths,
                    &instance_id,
                    auth_session.as_ref(),
                    &reporter,
                    dry_run,
                )
                .await
            };

            match launch_res {
                Ok(result) => {
                    if let Some(pid) = result.pid {
                        println!("Minecraft process running with PID {}", pid);
                    } else {
                        println!("Launch completed in dry-run mode.");
                    }
                }
                Err(SzzError::InstanceNotInstalled(id)) => {
                    eprintln!("Error: Instance '{}' is not installed.", id);
                    eprintln!("Run 'szz-cli install <manifest_path>' first to install this instance.");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Launch failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Update {
            instance_id,
            manifest,
        } => {
            let target_manifest = if let Some(path) = manifest {
                let m = ServerManifest::from_file(&path)?;
                if m.server_id != instance_id {
                    eprintln!(
                        "Error: Manifest server ID '{}' does not match target instance ID '{}'",
                        m.server_id, instance_id
                    );
                    std::process::exit(1);
                }
                m
            } else {
                // Fetch latest manifest from remote catalog
                println!("Fetching remote catalog to resolve latest manifest for '{}'...", instance_id);
                let catalog_url = szz_core::config::catalog_url();
                let cat_out = szz_core::catalog::fetch_catalog_default(&paths, &catalog_url).await?;
                let server_entry = cat_out
                    .catalog
                    .servers
                    .iter()
                    .find(|s| s.id == instance_id)
                    .ok_or_else(|| {
                        SzzError::Runtime(format!(
                            "Server '{}' not found in remote catalog",
                            instance_id
                        ))
                    })?;
                println!(
                    "Downloading latest manifest for '{}' (v{}) from {}...",
                    server_entry.name, server_entry.latest_manifest_version, server_entry.manifest_url
                );
                let mut m = szz_core::catalog::fetch_server_manifest_default(&server_entry.manifest_url).await?;
                m.server_id = instance_id.clone();
                m
            };

            println!(
                "Incrementally updating instance '{}' to manifest v{}...",
                target_manifest.server_id, target_manifest.manifest_version
            );

            match szz_core::instance::update_instance(&paths, &target_manifest, &reporter).await {
                Ok(_) => {
                    println!(
                        "Instance '{}' successfully updated to manifest v{}!",
                        target_manifest.server_id, target_manifest.manifest_version
                    );
                }
                Err(e) => {
                    eprintln!("Update failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Catalog { url } => {
            let target_url = url.unwrap_or_else(szz_core::config::catalog_url);
            println!("Fetching catalog from '{}'...", target_url);

            match szz_core::catalog::fetch_catalog_default(&paths, &target_url).await {
                Ok(result) => {
                    if result.from_cache {
                        println!("[OFFLINE] Displaying cached catalog (cached at ~/.szz/cache/catalog.json).");
                    }
                    println!(
                        "Catalog v{} (Updated: {}) - {} servers available:",
                        result.catalog.version,
                        result.catalog.updated_at.as_deref().unwrap_or("unknown"),
                        result.catalog.servers.len()
                    );
                    println!(
                        "{:<22} {:<28} {:<10} {:<18}",
                        "SERVER ID", "NAME", "VERSION", "STATUS"
                    );
                    println!("{}", "-".repeat(82));
                    for entry in result.catalog.servers {
                        let status = szz_core::instance::get_instance_status_with_remote(
                            &paths,
                            &entry.id,
                            Some(entry.latest_manifest_version),
                        )
                        .map(|s| format!("{:?}", s))
                        .unwrap_or_else(|_| "Unknown".to_string());

                        println!(
                            "{:<22} {:<28} v{:<9} {:<18}",
                            entry.id, entry.name, entry.latest_manifest_version, status
                        );
                        if !entry.tagline.is_empty() {
                            println!("  └─ {}", entry.tagline);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error fetching catalog: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Uninstall { instance_id } => {
            println!("Uninstalling instance '{}'...", instance_id);
            uninstall_instance(&paths, &instance_id)?;
            println!("Instance '{}' uninstalled successfully.", instance_id);
        }
        Commands::Settings { command } => match command {
            SettingsCommands::Status { key } => {
                let settings = szz_core::settings::UserSettings::load(&paths)?;
                if let Some(k) = key {
                    match settings.get_by_key(&k) {
                        Ok(val) => println!("{}", val),
                        Err(e) => {
                            eprintln!("Error: {}", e);
                            std::process::exit(1);
                        }
                    }
                } else {
                    let sys_mem = szz_core::settings::get_system_memory_info();
                    println!("SZZ Launcher Global Settings ({:?})", paths.settings_file());
                    println!("================================================================");
                    println!("[Game]");
                    println!(
                        "  game.resolution_width     = {}",
                        settings.game.resolution_width
                    );
                    println!(
                        "  game.resolution_height    = {}",
                        settings.game.resolution_height
                    );
                    println!(
                        "  game.fullscreen           = {}",
                        settings.game.fullscreen
                    );
                    println!(
                        "  game.auto_connect         = {}",
                        settings.game.auto_connect
                    );
                    println!("[Java & Memory]");
                    println!(
                        "  java.allocated_memory_mb  = {} MB (Host Total: {} MB, Avail: {} MB)",
                        settings.java.allocated_memory_mb,
                        sys_mem.total_memory_mb,
                        sys_mem.available_memory_mb
                    );
                    println!(
                        "  java.custom_jvm_args      = {}",
                        if settings.java.custom_jvm_args.is_empty() {
                            "(none)".to_string()
                        } else {
                            settings.java.custom_jvm_args.join(" ")
                        }
                    );
                    println!("[Launcher]");
                    println!(
                        "  launcher.behavior_on_launch = {}",
                        settings.launcher.behavior_on_launch.as_str()
                    );
                    println!("  launcher.theme            = {}", settings.launcher.theme);
                    println!("================================================================");
                }
            }
            SettingsCommands::Set { key, value } => {
                let mut settings = szz_core::settings::UserSettings::load(&paths)?;
                if let Err(e) = settings.set_by_key(&key, &value) {
                    eprintln!("Error updating setting '{}': {}", key, e);
                    std::process::exit(1);
                }
                settings.save(&paths)?;
                let updated = settings.get_by_key(&key).unwrap_or(value);
                println!("Updated '{}' = {}", key, updated);
            }
            SettingsCommands::Reset => {
                let settings = szz_core::settings::UserSettings::reset(&paths)?;
                println!(
                    "Settings reset to defaults (Memory: {} MB, Resolution: {}x{}).",
                    settings.java.allocated_memory_mb,
                    settings.game.resolution_width,
                    settings.game.resolution_height
                );
            }
            SettingsCommands::Storage { command } => {
                handle_storage_command(&paths, command)?;
            }
        },
        Commands::Ping { server_address } => {
            println!("Pinging Minecraft server '{}'...", server_address);
            let status = szz_core::ping::ping_server(
                &server_address,
                szz_core::ping::DEFAULT_PING_TIMEOUT,
            )
            .await;
            if status.online {
                println!("Status:  ONLINE");
                if let Some(ms) = status.latency_ms {
                    println!("Latency: {} ms", ms);
                }
                if let (Some(online), Some(max)) = (status.players_online, status.players_max) {
                    println!("Players: {} / {}", online, max);
                }
                if let Some(ver) = status.version_name {
                    println!("Version: {}", ver);
                }
                if let Some(motd) = status.motd {
                    println!("MOTD:    {}", motd);
                }
            } else {
                println!("Status:  OFFLINE (Server unreachable or timed out)");
            }
        }
        Commands::Storage { command } => {
            handle_storage_command(&paths, command)?;
        }
    }

    Ok(())
}

fn handle_storage_command(
    paths: &SzzPaths,
    command: StorageCommands,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        StorageCommands::Status => {
            let usage = szz_core::storage_gc::get_storage_usage(paths)?;
            println!("SZZ Storage Usage Overview");
            println!("================================================================================");
            println!("{:<16} {:<15} {:<15} {:<15} {:<15}", "Category", "Total Size", "Reclaimable", "Total Items", "Reclaimable");
            println!("--------------------------------------------------------------------------------");
            println!("{:<16} {:<15} {:<15} {:<15} {:<15}", "CAS Mods", format_bytes(usage.mods.total_bytes), format_bytes(usage.mods.reclaimable_bytes), usage.mods.item_count, usage.mods.reclaimable_count);
            println!("{:<16} {:<15} {:<15} {:<15} {:<15}", "Java Runtimes", format_bytes(usage.runtimes.total_bytes), format_bytes(usage.runtimes.reclaimable_bytes), usage.runtimes.item_count, usage.runtimes.reclaimable_count);
            println!("{:<16} {:<15} {:<15} {:<15} {:<15}", "Mojang Assets", format_bytes(usage.assets.total_bytes), format_bytes(usage.assets.reclaimable_bytes), usage.assets.item_count, usage.assets.reclaimable_count);
            println!("{:<16} {:<15} {:<15} {:<15} {:<15}", "Libraries", format_bytes(usage.libraries.total_bytes), format_bytes(usage.libraries.reclaimable_bytes), usage.libraries.item_count, usage.libraries.reclaimable_count);
            println!("{:<16} {:<15} {:<15} {:<15} {:<15}", "Instances", format_bytes(usage.instances.total_bytes), "0 B", usage.instances.item_count, 0);
            println!("--------------------------------------------------------------------------------");
            println!("{:<16} {:<15} {:<15}", "TOTAL", format_bytes(usage.total_bytes), format_bytes(usage.reclaimable_bytes));
            println!("================================================================================");
            if usage.reclaimable_bytes > 0 {
                println!("* {} of unused cache can be safely reclaimed.", format_bytes(usage.reclaimable_bytes));
                println!("* Run 'szz-cli settings storage clean' to free up disk space.");
            } else {
                println!("* All storage is optimal. No unused cache found.");
            }
        }
        StorageCommands::Clean {
            dry_run,
            only_mods,
            only_runtimes,
            only_assets,
            only_libraries,
        } => {
            let any_filter = only_mods || only_runtimes || only_assets || only_libraries;
            let options = szz_core::storage_gc::CleanOptions {
                clean_mods: if any_filter { only_mods } else { true },
                clean_runtimes: if any_filter { only_runtimes } else { true },
                clean_assets: if any_filter { only_assets } else { true },
                clean_libraries: if any_filter { only_libraries } else { true },
                dry_run,
            };

            if dry_run {
                println!("[DRY-RUN] Simulating storage cache cleanup...");
            } else {
                println!("Cleaning unused storage cache...");
            }

            match szz_core::storage_gc::clean_unused_cache(paths, &options) {
                Ok(summary) => {
                    if dry_run {
                        println!("[DRY-RUN] Estimated reclaimable space: {}", format_bytes(summary.reclaimed_bytes));
                        println!("  - Mods: {} item(s)", summary.deleted_mods_count);
                        println!("  - Java Runtimes: {} directory(ies)", summary.deleted_runtimes_count);
                        println!("  - Mojang Assets: {} object(s)", summary.deleted_assets_count);
                        println!("  - Libraries: {} file(s)", summary.deleted_libraries_count);
                        println!("[DRY-RUN] No files were deleted.");
                    } else {
                        println!("Storage cleanup completed successfully.");
                        println!("  - Reclaimed space: {}", format_bytes(summary.reclaimed_bytes));
                        println!(
                            "  - Removed: {} mods, {} runtimes, {} assets, {} libraries",
                            summary.deleted_mods_count,
                            summary.deleted_runtimes_count,
                            summary.deleted_assets_count,
                            summary.deleted_libraries_count
                        );
                    }
                }
                Err(SzzError::InstanceRunning(id)) => {
                    eprintln!("Error: Cannot clean storage cache while an instance is running ('{}').", id);
                    eprintln!("Please close the game client before performing maintenance.");
                    std::process::exit(1);
                }
                Err(SzzError::StorageGcInProgress) => {
                    eprintln!("Error: Another storage garbage collection process is currently running.");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Storage clean failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
