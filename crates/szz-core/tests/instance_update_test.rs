use sha1::{Digest, Sha1};
use szz_core::instance::{
    check_instance_update, get_instance_status_with_remote, update_instance,
    InstanceLifecycleStatus, InstanceMetadata,
};
use szz_core::manifest::{LoaderConfig, LoaderType, ModEntry, ServerManifest};
use szz_core::paths::SzzPaths;
use szz_core::progress::NoopProgressReporter;
use tempfile::TempDir;

fn sha1_of(bytes: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn make_manifest(
    id: &str,
    version: u32,
    mods: Vec<ModEntry>,
    config_url: Option<String>,
) -> ServerManifest {
    ServerManifest {
        manifest_version: version,
        server_id: id.to_string(),
        name: format!("Server {}", id),
        server_address: "127.0.0.1:25565".to_string(),
        minecraft_version: "1.20.1".to_string(),
        loader: LoaderConfig {
            loader_type: LoaderType::Fabric,
            version: "0.16.10".to_string(),
        },
        config: config_url.map(|u| szz_core::manifest::ConfigSpec {
            url: u,
            hash: None,
            version: None,
        }),
        mods,
    }
}

fn make_mod(name: &str, sha1: &str) -> ModEntry {
    ModEntry {
        name: name.to_string(),
        source: "modrinth".to_string(),
        file_url: format!("https://cdn.example.com/{}.jar", name),
        sha1: Some(sha1.to_string()),
        sha512: None,
    }
}

#[test]
fn test_check_instance_update_diff_calculation() {
    let mod_a = make_mod("mod-a", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let mod_b = make_mod("mod-b", "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let mod_c = make_mod("mod-c", "cccccccccccccccccccccccccccccccccccccccc");

    let v1 = make_manifest(
        "test-srv",
        1,
        vec![mod_a.clone(), mod_b.clone()],
        Some("https://example.com/cfg.zip".to_string()),
    );
    let v2 = make_manifest(
        "test-srv",
        2,
        vec![mod_b.clone(), mod_c.clone()],
        Some("https://example.com/cfg.zip".to_string()),
    );

    let diff = check_instance_update(&v1, &v2);

    assert_eq!(diff.old_manifest_version, 1);
    assert_eq!(diff.new_manifest_version, 2);
    assert!(diff.has_changes);
    assert_eq!(diff.added_mods.len(), 1);
    assert_eq!(diff.added_mods[0].name, "mod-c");
    assert_eq!(diff.removed_mods.len(), 1);
    assert_eq!(diff.removed_mods[0].name, "mod-a");
    assert_eq!(diff.unchanged_mods.len(), 1);
    assert_eq!(diff.unchanged_mods[0].name, "mod-b");
    assert!(diff.config_needs_update); // manifest version increased without explicit hash

    // When neither manifest defines config, config_needs_update must be false
    let v1_no_cfg = make_manifest("test-srv", 1, vec![mod_a], None);
    let v2_no_cfg = make_manifest("test-srv", 2, vec![mod_c], None);
    let diff_no_cfg = check_instance_update(&v1_no_cfg, &v2_no_cfg);
    assert!(!diff_no_cfg.config_needs_update);
}

#[test]
fn test_get_instance_status_with_remote_detects_update() {
    let temp = TempDir::new().unwrap();
    let paths = SzzPaths::custom(temp.path());

    let mod_a = make_mod("mod-a", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let v1 = make_manifest("demo-srv", 1, vec![mod_a], None);

    let instance_dir = paths.instance_dir(&v1.server_id);
    std::fs::create_dir_all(&instance_dir).unwrap();
    let meta = InstanceMetadata::new("demo-srv", v1);
    meta.save_to_dir(&instance_dir).unwrap();

    // 1. Same manifest version -> Installed
    let status_same = get_instance_status_with_remote(&paths, "demo-srv", Some(1)).unwrap();
    assert_eq!(status_same, InstanceLifecycleStatus::Installed);

    // 2. Newer manifest version -> UpdateAvailable
    let status_newer = get_instance_status_with_remote(&paths, "demo-srv", Some(2)).unwrap();
    assert_eq!(status_newer, InstanceLifecycleStatus::UpdateAvailable);

    // 3. None specified -> Installed
    let status_none = get_instance_status_with_remote(&paths, "demo-srv", None).unwrap();
    assert_eq!(status_none, InstanceLifecycleStatus::Installed);
}

#[tokio::test]
async fn test_update_instance_removes_old_mods_and_updates_metadata() {
    let temp = TempDir::new().unwrap();
    let paths = SzzPaths::custom(temp.path());
    let reporter = NoopProgressReporter;

    let mod_a = make_mod("mod-a", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let mod_b = make_mod("mod-b", "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");

    // Pre-populate installed instance v1 with dummy jar files in mods/
    let v1 = make_manifest("upgrade-srv", 1, vec![mod_a.clone(), mod_b.clone()], None);
    let instance_dir = paths.instance_dir(&v1.server_id);
    let mods_dir = paths.instance_mods_dir(&v1.server_id);
    std::fs::create_dir_all(&mods_dir).unwrap();

    let meta = InstanceMetadata::new("upgrade-srv", v1);
    meta.save_to_dir(&instance_dir).unwrap();

    let mod_a_file = mods_dir.join(mod_a.filename());
    let mod_b_file = mods_dir.join(mod_b.filename());
    std::fs::write(&mod_a_file, b"dummy mod a content").unwrap();
    std::fs::write(&mod_b_file, b"dummy mod b content").unwrap();

    assert!(mod_a_file.is_file());
    assert!(mod_b_file.is_file());

    // Target manifest v2: mod_a removed, mod_b unchanged, no new mod to download
    let v2 = make_manifest("upgrade-srv", 2, vec![mod_b.clone()], None);

    let diff = update_instance(&paths, &v2, &reporter)
        .await
        .expect("update_instance should succeed");

    assert_eq!(diff.removed_mods.len(), 1);
    assert_eq!(diff.removed_mods[0].name, "mod-a");

    // Old mod-a file must be removed from mods/
    assert!(!mod_a_file.exists(), "mod-a file must be removed");
    // Mod-b file must still exist
    assert!(mod_b_file.exists(), "mod-b file must remain intact");

    // Metadata must be atomically committed to v2
    let updated_meta = InstanceMetadata::load_from_dir(&instance_dir)
        .unwrap()
        .expect("meta must exist");
    assert_eq!(updated_meta.manifest.manifest_version, 2);
    assert_eq!(updated_meta.manifest.mods.len(), 1);
    assert_eq!(updated_meta.manifest.mods[0].name, "mod-b");
}

#[tokio::test]
async fn test_update_instance_same_filename_different_hash_retains_mod() {
    let temp = TempDir::new().unwrap();
    let paths = SzzPaths::custom(temp.path());
    let reporter = NoopProgressReporter;

    // Create CAS store directory
    std::fs::create_dir_all(paths.store_dir()).unwrap();

    // Mod V1: same filename "cloth-config.jar", hash sha1_v1
    let v1_content = b"cloth-config-v1-bytes";
    let sha1_v1 = sha1_of(v1_content);
    let mod_v1 = ModEntry {
        name: "cloth-config".to_string(),
        source: "modrinth".to_string(),
        file_url: "https://cdn.example.com/cloth-config.jar".to_string(),
        sha1: Some(sha1_v1.clone()),
        sha512: None,
    };

    // Store v1 in CAS
    let store_v1 = paths.store_dir().join(format!("{}.jar", sha1_v1));
    std::fs::write(&store_v1, v1_content).unwrap();

    // Mod V2: same filename "cloth-config.jar", hash sha1_v2
    let v2_content = b"cloth-config-v2-bytes-updated";
    let sha1_v2 = sha1_of(v2_content);
    let mod_v2 = ModEntry {
        name: "cloth-config".to_string(),
        source: "modrinth".to_string(),
        file_url: "https://cdn.example.com/cloth-config.jar".to_string(),
        sha1: Some(sha1_v2.clone()),
        sha512: None,
    };

    // Store v2 in CAS
    let store_v2 = paths.store_dir().join(format!("{}.jar", sha1_v2));
    std::fs::write(&store_v2, v2_content).unwrap();

    // Initialize installed instance with v1
    let v1_manifest = make_manifest("same-name-srv", 1, vec![mod_v1], None);
    let instance_dir = paths.instance_dir(&v1_manifest.server_id);
    let mods_dir = paths.instance_mods_dir(&v1_manifest.server_id);
    std::fs::create_dir_all(&mods_dir).unwrap();

    let meta = InstanceMetadata::new("same-name-srv", v1_manifest);
    meta.save_to_dir(&instance_dir).unwrap();

    // Link v1 to mods/cloth-config.jar
    let mod_jar = mods_dir.join("cloth-config.jar");
    std::fs::write(&mod_jar, v1_content).unwrap();

    // Target manifest with v2 (upgraded version with identical jar name)
    let v2_manifest = make_manifest("same-name-srv", 2, vec![mod_v2], None);

    let diff = update_instance(&paths, &v2_manifest, &reporter)
        .await
        .expect("update_instance should succeed");

    assert_eq!(diff.removed_mods.len(), 1);
    assert_eq!(diff.added_mods.len(), 1);
    assert_eq!(diff.removed_mods[0].filename(), "cloth-config.jar");
    assert_eq!(diff.added_mods[0].filename(), "cloth-config.jar");

    // CRITICAL: cloth-config.jar must exist and contain the updated v2 content!
    assert!(mod_jar.exists(), "cloth-config.jar must NOT be deleted by removed_mods cleanup");
    let content = std::fs::read(&mod_jar).unwrap();
    assert_eq!(content, v2_content, "cloth-config.jar must be updated to v2 content");

    // Also verify CAS store still has both v1 and v2
    assert!(store_v2.exists(), "CAS store must retain v2");
}
