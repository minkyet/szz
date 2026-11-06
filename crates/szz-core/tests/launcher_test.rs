use std::path::PathBuf;
use szz_core::launcher::{build_command, format_command_line, LaunchConfig};

#[test]
fn test_build_command_arguments() {
    let config = LaunchConfig {
        java_binary: PathBuf::from("/mock/bin/java"),
        game_dir: PathBuf::from("/mock/servers/srv-1"),
        assets_dir: PathBuf::from("/mock/assets"),
        asset_index: "1.20".to_string(),
        version: "1.20.1".to_string(),
        main_class: "net.fabricmc.loader.impl.launch.knot.KnotClient".to_string(),
        classpath: "lib1.jar:lib2.jar:client.jar".to_string(),
        max_memory: "-Xmx2G".to_string(),
        username: "DevTester".to_string(),
        uuid: "00000000-0000-0000-0000-000000000000".to_string(),
        access_token: "token123".to_string(),
        user_type: "msa".to_string(),
        quick_play_multiplayer: Some("play.example.com:25565".to_string()),
        extra_jvm_args: Vec::new(),
        extra_game_args: Vec::new(),
    };

    let cmd = build_command(&config);
    let cmd_str = format_command_line(&cmd);

    assert!(cmd_str.contains("/mock/bin/java"));
    assert!(cmd_str.contains("-Xmx2G"));
    assert!(cmd_str.contains("-cp lib1.jar:lib2.jar:client.jar"));
    assert!(cmd_str.contains("net.fabricmc.loader.impl.launch.knot.KnotClient"));
    assert!(cmd_str.contains("--version 1.20.1"));
    assert!(cmd_str.contains("--assetsDir /mock/assets"));
    assert!(cmd_str.contains("--assetIndex 1.20"));
    assert!(cmd_str.contains("--username DevTester"));
    assert!(cmd_str.contains("--quickPlayMultiplayer play.example.com:25565"));
}

#[test]
fn test_build_command_pre_1_20_server_branching() {
    let config = LaunchConfig {
        java_binary: PathBuf::from("/mock/bin/java"),
        game_dir: PathBuf::from("/mock/servers/srv-1"),
        assets_dir: PathBuf::from("/mock/assets"),
        asset_index: "1.12".to_string(),
        version: "1.12.2".to_string(),
        main_class: "net.minecraft.launchwrapper.Launch".to_string(),
        classpath: "forge.jar:client.jar".to_string(),
        max_memory: "-Xmx2G".to_string(),
        username: "DevTester".to_string(),
        uuid: "00000000-0000-0000-0000-000000000000".to_string(),
        access_token: "token123".to_string(),
        user_type: "msa".to_string(),
        quick_play_multiplayer: Some("play.example.com:25565".to_string()),
        extra_jvm_args: vec!["-Dfml.ignoreInvalidMinecraftCertificates=true".to_string()],
        extra_game_args: vec![
            "--tweakClass".to_string(),
            "net.minecraftforge.fml.common.launcher.FMLTweaker".to_string(),
        ],
    };

    let cmd = build_command(&config);
    let cmd_str = format_command_line(&cmd);

    assert!(cmd_str.contains("--assetIndex 1.12"));
    assert!(cmd_str.contains("--assetsDir /mock/assets"));

    // In 1.12.2, --server and --port must be used instead of --quickPlayMultiplayer
    assert!(!cmd_str.contains("--quickPlayMultiplayer"));
    assert!(cmd_str.contains("--server play.example.com"));
    assert!(cmd_str.contains("--port 25565"));
    assert!(cmd_str.contains("-Dfml.ignoreInvalidMinecraftCertificates=true"));
    assert!(cmd_str.contains("--tweakClass net.minecraftforge.fml.common.launcher.FMLTweaker"));
    assert!(cmd_str.contains("net.minecraft.launchwrapper.Launch"));
    assert!(cmd_str.contains("--accessToken [REDACTED]"));
    assert!(!cmd_str.contains("token123"));
}

#[test]
fn test_progress_reporter() {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use szz_core::progress::{LaunchStep, ProgressReporter};

    struct TestReporter(Arc<AtomicU32>);
    impl ProgressReporter for TestReporter {
        fn report(&self, _step: LaunchStep, _msg: Option<&str>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    let count = Arc::new(AtomicU32::new(0));
    let reporter = TestReporter(count.clone());
    reporter.report(LaunchStep::PreparingInstance, None);
    reporter.report(LaunchStep::Complete, None);
    assert_eq!(count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_sync_server_config_when_none() {
    use szz_core::config_sync::sync_server_config;
    use szz_core::manifest::ServerManifest;
    use szz_core::paths::SzzPaths;
    use tempfile::tempdir;

    let tmp = tempdir().unwrap();
    let paths = SzzPaths::custom(tmp.path().join(".szz"));
    let manifest_json = r#"{
        "serverId": "test-srv",
        "name": "Test Server",
        "serverAddress": "mc.example.com:25565",
        "minecraftVersion": "1.20.1",
        "loader": { "type": "fabric", "version": "0.15.11" },
        "config": null,
        "manifestVersion": 1,
        "mods": []
    }"#;
    let manifest = ServerManifest::from_json_str(manifest_json).unwrap();
    let res = sync_server_config(&paths, &manifest, false).await.unwrap();
    assert!(res.is_none());
}

#[test]
fn test_apply_settings_to_launch_config() {
    use szz_core::launcher::apply_settings_to_launch_config;
    use szz_core::settings::UserSettings;

    let mut config = LaunchConfig {
        java_binary: PathBuf::from("/mock/bin/java"),
        game_dir: PathBuf::from("/mock/servers/srv-1"),
        assets_dir: PathBuf::from("/mock/assets"),
        asset_index: "1.20".to_string(),
        version: "1.20.1".to_string(),
        main_class: "net.fabricmc.loader.impl.launch.knot.KnotClient".to_string(),
        classpath: "lib1.jar:client.jar".to_string(),
        max_memory: "-Xmx2G".to_string(),
        username: "DevTester".to_string(),
        uuid: "00000000-0000-0000-0000-000000000000".to_string(),
        access_token: "token123".to_string(),
        user_type: "msa".to_string(),
        quick_play_multiplayer: Some("play.example.com:25565".to_string()),
        extra_jvm_args: Vec::new(),
        extra_game_args: Vec::new(),
    };

    let mut settings = UserSettings::default();
    settings.java.allocated_memory_mb = 6144;
    settings.java.custom_jvm_args = vec!["-XX:+UseG1GC".to_string()];
    settings.game.resolution_width = 1920;
    settings.game.resolution_height = 1080;
    settings.game.fullscreen = true;
    settings.game.auto_connect = false;

    apply_settings_to_launch_config(&mut config, &settings, "play.example.com:25565");

    let cmd = build_command(&config);
    let cmd_str = format_command_line(&cmd);

    assert!(cmd_str.contains("-Xmx6144M"));
    assert!(cmd_str.contains("-XX:+UseG1GC"));
    assert!(cmd_str.contains("--fullscreen"));
    assert!(cmd_str.contains("--width 1920"));
    assert!(cmd_str.contains("--height 1080"));
    assert!(!cmd_str.contains("--quickPlayMultiplayer"));
}

#[tokio::test]
async fn test_offline_server_launch_blocked_when_not_dry_run() {
    use szz_core::auth::DevAuthSession;
    use szz_core::error::SzzError;
    use szz_core::launcher::launch_server_manifest;
    use szz_core::manifest::ServerManifest;
    use szz_core::paths::SzzPaths;
    use szz_core::progress::NoopProgressReporter;
    use tempfile::tempdir;

    let tmp = tempdir().unwrap();
    let paths = SzzPaths::custom(tmp.path());
    let auth = DevAuthSession::new("Tester");
    let reporter = NoopProgressReporter;

    // Pre-populate mock Java 17 binary to prevent network access during dry-run in hermetic environments
    let java_bin = paths.java_binary_path(17);
    std::fs::create_dir_all(java_bin.parent().unwrap()).unwrap();
    std::fs::write(&java_bin, b"mock-java").unwrap();

    // Use a port where no server is running
    let manifest_json = r#"{
        "serverId": "offline-srv",
        "name": "Offline Server",
        "serverAddress": "127.0.0.1:54321",
        "minecraftVersion": "1.20.1",
        "loader": { "type": "fabric", "version": "0.15.11" },
        "manifestVersion": 1,
        "mods": []
    }"#;
    let manifest = ServerManifest::from_json_str(manifest_json).unwrap();

    // 1. Dry run should NOT be blocked by offline server
    let dry_run_res = launch_server_manifest(&paths, &manifest, &auth, &reporter, true).await;
    assert!(dry_run_res.is_ok());

    // 2. Real launch (dry_run: false) must be rejected with ServerOffline
    let real_res = launch_server_manifest(&paths, &manifest, &auth, &reporter, false).await;
    assert!(real_res.is_err());
    match real_res.err().unwrap() {
        SzzError::ServerOffline(addr) => {
            assert_eq!(addr, "127.0.0.1:54321");
        }
        other => panic!("Expected SzzError::ServerOffline, got {:?}", other),
    }
}
