use szz_core::instance::{
    get_instance_status, install_instance, list_installed_instances, uninstall_instance,
    validate_instance_id, InstanceLifecycleStatus,
};
use szz_core::launcher::launch_instance;
use szz_core::manifest::ServerManifest;
use szz_core::paths::SzzPaths;
use szz_core::progress::NoopProgressReporter;
use szz_core::auth::DevAuthSession;
use tempfile::TempDir;

fn sample_manifest() -> ServerManifest {
    let fixture_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/sample-manifest.json");
    ServerManifest::from_file(&fixture_path).expect("sample manifest should parse")
}

#[tokio::test]
async fn test_instance_lifecycle_not_installed() {
    let temp = TempDir::new().unwrap();
    let paths = SzzPaths::custom(temp.path());

    let status = get_instance_status(&paths, "nonexistent").unwrap();
    assert_eq!(status, InstanceLifecycleStatus::NotInstalled);

    let auth = DevAuthSession::new("TestPlayer");
    let reporter = NoopProgressReporter;
    let err = launch_instance(&paths, "nonexistent", &auth, &reporter, true).await.unwrap_err();
    assert!(err.to_string().contains("Instance not installed"));
}

#[tokio::test]
async fn test_instance_install_dry_run() {
    let temp = TempDir::new().unwrap();
    let paths = SzzPaths::custom(temp.path());
    let manifest = sample_manifest();
    let reporter = NoopProgressReporter;

    // dry-run does not write .szz_instance.json to disk
    install_instance(&paths, &manifest, &reporter, true)
        .await
        .expect("dry run install should succeed");

    let status = get_instance_status(&paths, &manifest.server_id).unwrap();
    assert_eq!(status, InstanceLifecycleStatus::NotInstalled);
}

#[tokio::test]
async fn test_instance_install_and_list_and_uninstall() {
    let temp = TempDir::new().unwrap();
    let paths = SzzPaths::custom(temp.path());
    let manifest = sample_manifest();
    let reporter = NoopProgressReporter;

    // Install live (using mock/fake fast install)
    let instance_dir = paths.ensure_instance_dirs(&manifest.server_id).unwrap();
    let meta = szz_core::instance::InstanceMetadata::new(&manifest.server_id, manifest.clone());
    meta.save_to_dir(&instance_dir).unwrap();

    let status = get_instance_status(&paths, &manifest.server_id).unwrap();
    assert_eq!(status, InstanceLifecycleStatus::Installed);

    let installed_list = list_installed_instances(&paths).unwrap();
    assert_eq!(installed_list.len(), 1);
    assert_eq!(installed_list[0].instance_id, manifest.server_id);
    assert_eq!(installed_list[0].server_name, manifest.name);
    assert_eq!(installed_list[0].status, InstanceLifecycleStatus::Installed);

    // Launch dry-run with installed instance
    let auth = DevAuthSession::new("TestPlayer");
    let result = launch_instance(&paths, &manifest.server_id, &auth, &reporter, true)
        .await
        .expect("dry-run launch should succeed for installed instance");
    assert!(result.command_line.contains("net.fabricmc.loader.impl.launch.knot.KnotClient"));

    // Uninstall
    uninstall_instance(&paths, &manifest.server_id).unwrap();
    let status_after = get_instance_status(&paths, &manifest.server_id).unwrap();
    assert_eq!(status_after, InstanceLifecycleStatus::NotInstalled);

    let empty_list = list_installed_instances(&paths).unwrap();
    assert!(empty_list.is_empty());
}

#[tokio::test]
async fn test_instance_id_security_validation() {
    let temp = TempDir::new().unwrap();
    let paths = SzzPaths::custom(temp.path());

    // Valid IDs
    assert!(validate_instance_id("valid-instance_123").is_ok());
    assert!(validate_instance_id("server-1").is_ok());

    // Invalid IDs (Path Traversal, Special Chars, Empty)
    assert!(validate_instance_id("").is_err());
    assert!(validate_instance_id("   ").is_err());
    assert!(validate_instance_id("../escape").is_err());
    assert!(validate_instance_id("foo/bar").is_err());
    assert!(validate_instance_id("foo\\bar").is_err());
    assert!(validate_instance_id("instance*1").is_err());

    // uninstall_instance must reject path traversal
    let err = uninstall_instance(&paths, "../escape").unwrap_err();
    assert!(matches!(err, szz_core::error::SzzError::Security(_)));

    // get_instance_status must reject path traversal
    let err2 = get_instance_status(&paths, "bad/id").unwrap_err();
    assert!(matches!(err2, szz_core::error::SzzError::Security(_)));

    // ensure_instance_dirs must reject path traversal
    let err3 = paths.ensure_instance_dirs("bad\\id").unwrap_err();
    assert!(matches!(err3, szz_core::error::SzzError::Security(_)));

    // launch_instance must reject path traversal
    let auth = DevAuthSession::new("TestPlayer");
    let reporter = NoopProgressReporter;
    let err4 = launch_instance(&paths, "../escape", &auth, &reporter, true).await.unwrap_err();
    assert!(matches!(err4, szz_core::error::SzzError::Security(_)));
}

#[test]
fn test_instance_lifecycle_status_serialization_and_pid() {
    let temp = TempDir::new().unwrap();
    let paths = SzzPaths::custom(temp.path());

    // Test serialization: must be PascalCase
    let status = InstanceLifecycleStatus::Installed;
    let json = serde_json::to_string(&status).unwrap();
    assert_eq!(json, "\"Installed\"");

    // Test deserialization: accepts PascalCase and camelCase
    let de1: InstanceLifecycleStatus = serde_json::from_str("\"Installed\"").unwrap();
    assert_eq!(de1, InstanceLifecycleStatus::Installed);
    let de2: InstanceLifecycleStatus = serde_json::from_str("\"installed\"").unwrap();
    assert_eq!(de2, InstanceLifecycleStatus::Installed);
    let de3: InstanceLifecycleStatus = serde_json::from_str("\"running\"").unwrap();
    assert_eq!(de3, InstanceLifecycleStatus::Running);

    // Test PID-based Running detection
    let instance_dir = paths.ensure_instance_dirs("test-server").unwrap();
    let meta = szz_core::instance::InstanceMetadata::new("test-server", sample_manifest());
    meta.save_to_dir(&instance_dir).unwrap();

    assert_eq!(get_instance_status(&paths, "test-server").unwrap(), InstanceLifecycleStatus::Installed);

    // Current process PID is alive
    let current_pid = std::process::id();
    let pid_file = instance_dir.join(".szz_instance.pid");
    std::fs::write(&pid_file, current_pid.to_string()).unwrap();

    assert_eq!(get_instance_status(&paths, "test-server").unwrap(), InstanceLifecycleStatus::Running);

    // Dead PID (e.g. 9999999)
    std::fs::write(&pid_file, "9999999").unwrap();
    assert_eq!(get_instance_status(&paths, "test-server").unwrap(), InstanceLifecycleStatus::Installed);
    assert!(!pid_file.exists()); // Stale PID file is automatically pruned
}

