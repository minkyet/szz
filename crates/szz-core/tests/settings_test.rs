use szz_core::paths::SzzPaths;
use szz_core::settings::{
    get_system_memory_info, LauncherBehaviorOnLaunch, SystemMemoryInfo, UserSettings,
    DEFAULT_MEMORY_MB, DEFAULT_RESOLUTION_HEIGHT, DEFAULT_RESOLUTION_WIDTH, MIN_MEMORY_MB,
};
use tempfile::TempDir;

#[test]
fn test_settings_load_creates_default_when_missing() {
    let temp = TempDir::new().unwrap();
    let paths = SzzPaths::custom(temp.path());

    assert!(!paths.settings_file().exists());

    let settings = UserSettings::load(&paths).expect("should create and return defaults");
    assert!(paths.settings_file().exists());
    assert_eq!(settings.game.resolution_width, DEFAULT_RESOLUTION_WIDTH);
    assert_eq!(settings.game.resolution_height, DEFAULT_RESOLUTION_HEIGHT);
    assert!(!settings.game.fullscreen);
    assert!(settings.game.auto_connect);
    assert_eq!(settings.java.allocated_memory_mb, DEFAULT_MEMORY_MB);
    assert!(settings.java.custom_jvm_args.is_empty());
    assert_eq!(
        settings.launcher.behavior_on_launch,
        LauncherBehaviorOnLaunch::KeepOpen
    );
    assert_eq!(settings.launcher.theme, "latte");
}

#[test]
fn test_settings_save_and_reload_persists_changes() {
    let temp = TempDir::new().unwrap();
    let paths = SzzPaths::custom(temp.path());

    let mut settings = UserSettings::load(&paths).unwrap();
    settings.game.resolution_width = 1920;
    settings.game.resolution_height = 1080;
    settings.game.fullscreen = true;
    settings.game.auto_connect = false;
    settings.java.allocated_memory_mb = 6144;
    settings.java.custom_jvm_args = vec!["-XX:+UseG1GC".to_string(), "-Xmn256M".to_string()];
    settings.launcher.behavior_on_launch = LauncherBehaviorOnLaunch::Minimize;
    settings.launcher.theme = "dark".to_string();

    settings.save(&paths).expect("should save settings atomically");

    let reloaded = UserSettings::load(&paths).expect("should reload saved settings");
    assert_eq!(reloaded.game.resolution_width, 1920);
    assert_eq!(reloaded.game.resolution_height, 1080);
    assert!(reloaded.game.fullscreen);
    assert!(!reloaded.game.auto_connect);
    assert_eq!(reloaded.java.allocated_memory_mb, 6144);
    assert_eq!(
        reloaded.java.custom_jvm_args,
        vec!["-XX:+UseG1GC".to_string(), "-Xmn256M".to_string()]
    );
    assert_eq!(
        reloaded.launcher.behavior_on_launch,
        LauncherBehaviorOnLaunch::Minimize
    );
    assert_eq!(reloaded.launcher.theme, "dark");
}

#[test]
fn test_settings_malformed_json_recovers_to_default() {
    let temp = TempDir::new().unwrap();
    let paths = SzzPaths::custom(temp.path());
    std::fs::write(paths.settings_file(), "{ broken json !!!").unwrap();

    let recovered = UserSettings::load(&paths).expect("should recover gracefully");
    assert_eq!(recovered.java.allocated_memory_mb, DEFAULT_MEMORY_MB);
    assert_eq!(recovered.game.resolution_width, DEFAULT_RESOLUTION_WIDTH);

    // File should be overwritten with valid JSON
    let raw = std::fs::read_to_string(paths.settings_file()).unwrap();
    assert!(serde_json::from_str::<UserSettings>(&raw).is_ok());
}

#[test]
fn test_settings_sanitize_clamps_bounds() {
    let mut settings = UserSettings::default();
    settings.java.allocated_memory_mb = 256; // Below MIN_MEMORY_MB
    settings.game.resolution_width = 100; // Below 320
    settings.game.resolution_height = 50; // Below 240
    settings.launcher.theme = "invalid_theme".to_string();

    let mock_mem = SystemMemoryInfo {
        total_memory_mb: 8192,
        available_memory_mb: 4096,
    };
    settings.sanitize(&mock_mem);

    assert_eq!(settings.java.allocated_memory_mb, MIN_MEMORY_MB);
    assert_eq!(settings.game.resolution_width, DEFAULT_RESOLUTION_WIDTH);
    assert_eq!(settings.game.resolution_height, DEFAULT_RESOLUTION_HEIGHT);
    assert_eq!(settings.launcher.theme, "latte");

    // Test upper bound (90% of 8192 = 7372)
    settings.java.allocated_memory_mb = 32768;
    settings.sanitize(&mock_mem);
    assert_eq!(settings.java.allocated_memory_mb, 7372);
}

#[test]
fn test_settings_get_and_set_by_key() {
    let mut settings = UserSettings::default();
    settings
        .set_by_key("java.allocated_memory_mb", "6144")
        .unwrap();
    assert_eq!(
        settings.get_by_key("java.allocated_memory_mb").unwrap(),
        "6144"
    );

    settings.set_by_key("game.auto_connect", "false").unwrap();
    assert_eq!(settings.get_by_key("game.auto_connect").unwrap(), "false");

    settings
        .set_by_key("launcher.behavior_on_launch", "minimize")
        .unwrap();
    assert_eq!(
        settings.get_by_key("launcher.behavior_on_launch").unwrap(),
        "minimize"
    );

    assert!(settings.set_by_key("unknown.key", "123").is_err());
    assert!(settings
        .set_by_key("java.allocated_memory_mb", "512")
        .is_err());
}

#[test]
fn test_get_system_memory_info_returns_valid_numbers() {
    let mem = get_system_memory_info();
    assert!(mem.total_memory_mb >= 2048);
    assert!(mem.available_memory_mb >= 512);
    assert!(mem.available_memory_mb <= mem.total_memory_mb);
}
