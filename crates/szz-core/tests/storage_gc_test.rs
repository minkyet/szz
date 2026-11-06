use std::fs;
use szz_core::auth::DevAuthSession;
use szz_core::error::SzzError;
use szz_core::instance::{
    get_instance_status, install_instance, uninstall_instance, InstanceLifecycleStatus,
    InstanceMetadata,
};
use szz_core::launcher::launch_instance;
use szz_core::manifest::{LoaderConfig, LoaderType, ModEntry, ServerManifest};
use szz_core::paths::SzzPaths;
use szz_core::progress::NoopProgressReporter;
use szz_core::storage_gc::{
    clean_unused_cache, get_storage_usage, is_gc_in_progress, CleanOptions, GcLockGuard,
};
use szz_core::store::{file_hardlink_count, prune_orphaned_mods};
use tempfile::TempDir;

static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn dummy_manifest(server_id: &str, mc_version: &str, mods: Vec<ModEntry>) -> ServerManifest {
    ServerManifest {
        manifest_version: 1,
        server_id: server_id.to_string(),
        name: format!("Server {}", server_id),
        server_address: "127.0.0.1:25565".to_string(),
        minecraft_version: mc_version.to_string(),
        loader: LoaderConfig {
            loader_type: LoaderType::Fabric,
            version: "0.15.11".to_string(),
        },
        config: None,
        mods,
    }
}

#[test]
fn test_shared_cas_mod_retention_and_pruning_on_uninstall() {
    let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let temp = TempDir::new().unwrap();
    let paths = SzzPaths::custom(temp.path());

    // 1. Create CAS store and put a shared mod file in it
    fs::create_dir_all(paths.store_dir()).unwrap();
    let mod_filename = "shared-mod-1234567890abcdef.jar";
    let store_jar = paths.store_dir().join(mod_filename);
    let dummy_jar_bytes = vec![0xCA, 0xFE, 0xBA, 0xBE, 0x01, 0x02, 0x03, 0x04];
    fs::write(&store_jar, &dummy_jar_bytes).unwrap();

    let mod_entry = ModEntry {
        name: "shared-mod".to_string(),
        source: "custom".to_string(),
        file_url: "https://example.com/shared-mod.jar".to_string(),
        sha1: Some("1234567890abcdef".to_string()),
        sha512: None,
    };

    // 2. Set up Instance A and Instance B
    let instance_a_id = "server-alpha";
    let instance_b_id = "server-beta";

    let manifest_a = dummy_manifest(instance_a_id, "1.20.1", vec![mod_entry.clone()]);
    let manifest_b = dummy_manifest(instance_b_id, "1.20.1", vec![mod_entry.clone()]);

    let dir_a = paths.ensure_instance_dirs(instance_a_id).unwrap();
    let dir_b = paths.ensure_instance_dirs(instance_b_id).unwrap();

    InstanceMetadata::new(instance_a_id, manifest_a)
        .save_to_dir(&dir_a)
        .unwrap();
    InstanceMetadata::new(instance_b_id, manifest_b)
        .save_to_dir(&dir_b)
        .unwrap();

    // 3. Hardlink from store to instance mods directories
    let mod_link_a = paths.instance_mods_dir(instance_a_id).join("shared-mod.jar");
    let mod_link_b = paths.instance_mods_dir(instance_b_id).join("shared-mod.jar");
    fs::hard_link(&store_jar, &mod_link_a).unwrap();
    fs::hard_link(&store_jar, &mod_link_b).unwrap();

    // Verify initial hardlink count is 3 (store + instance A + instance B)
    assert_eq!(file_hardlink_count(&store_jar).unwrap(), 3);

    // 4. Uninstall Instance A
    uninstall_instance(&paths, instance_a_id).expect("uninstall alpha should succeed");

    // The shared mod must STILL exist in store because Instance B still links to it!
    assert!(store_jar.exists(), "store jar must remain while instance B is still installed");
    assert_eq!(file_hardlink_count(&store_jar).unwrap(), 2);
    assert!(mod_link_b.exists());

    // 5. Uninstall Instance B
    uninstall_instance(&paths, instance_b_id).expect("uninstall beta should succeed");

    // Now hardlink count was 1, so prune_orphaned_mods automatically purged it!
    assert!(!store_jar.exists(), "store jar must be pruned once no instance references it");
}

#[test]
fn test_orphaned_cas_mod_pruning_standalone() {
    let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let temp = TempDir::new().unwrap();
    let paths = SzzPaths::custom(temp.path());

    fs::create_dir_all(paths.store_dir()).unwrap();
    let orphan_jar = paths.store_dir().join("orphan-hash.jar");
    fs::write(&orphan_jar, b"dummy-orphan-content-12345").unwrap();

    // Standalone prune call
    let summary = prune_orphaned_mods(&paths).expect("prune should succeed");
    assert_eq!(summary.pruned_count, 1);
    assert_eq!(summary.reclaimed_bytes, 26);
    assert!(!orphan_jar.exists());

    // Second prune call should report 0
    let summary2 = prune_orphaned_mods(&paths).expect("prune should succeed");
    assert_eq!(summary2.pruned_count, 0);
    assert_eq!(summary2.reclaimed_bytes, 0);
}

#[test]
fn test_clean_unused_cache_runtimes_and_assets() {
    let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let temp = TempDir::new().unwrap();
    let paths = SzzPaths::custom(temp.path());

    // 1. Create instances: one 1.20.1 (Java 17) and one 1.21.1 (Java 21)
    let inst1_dir = paths.ensure_instance_dirs("inst-17").unwrap();
    let inst2_dir = paths.ensure_instance_dirs("inst-21").unwrap();

    let meta1 = dummy_manifest("inst-17", "1.20.1", vec![]);
    let meta2 = dummy_manifest("inst-21", "1.21.1", vec![]);
    InstanceMetadata::new("inst-17", meta1).save_to_dir(&inst1_dir).unwrap();
    InstanceMetadata::new("inst-21", meta2).save_to_dir(&inst2_dir).unwrap();

    // 2. Set up Runtimes: java-17, java-21 (used) and java-8, java-11 (unused)
    let runtimes = ["java-8", "java-11", "java-17", "java-21"];
    for rt in &runtimes {
        let rt_dir = paths.runtimes_dir().join(rt).join("bin");
        fs::create_dir_all(&rt_dir).unwrap();
        fs::write(rt_dir.join("java"), b"mock-java").unwrap();
    }

    // 3. Set up Assets:
    // 1.20.json (used by inst-17)
    let index_1_20_path = paths.asset_index_path("1.20");
    fs::create_dir_all(index_1_20_path.parent().unwrap()).unwrap();
    let index_1_20 = r#"{
        "objects": {
            "icons/app.png": { "hash": "aaaabbbb11112222", "size": 10 }
        }
    }"#;
    fs::write(&index_1_20_path, index_1_20).unwrap();

    // 1.16.json (unused by any installed instance)
    let index_1_16_path = paths.asset_index_path("1.16");
    let index_1_16 = r#"{
        "objects": {
            "sounds/music.ogg": { "hash": "ccccdddd33334444", "size": 20 }
        }
    }"#;
    fs::write(&index_1_16_path, index_1_16).unwrap();

    // Object blobs
    let obj_a = paths.asset_object_path("aaaabbbb11112222");
    fs::create_dir_all(obj_a.parent().unwrap()).unwrap();
    fs::write(&obj_a, b"0123456789").unwrap();

    let obj_c = paths.asset_object_path("ccccdddd33334444");
    fs::create_dir_all(obj_c.parent().unwrap()).unwrap();
    fs::write(&obj_c, b"01234567890123456789").unwrap();

    // 4. Query storage usage report
    let report = get_storage_usage(&paths).expect("report should succeed");
    assert_eq!(report.runtimes.item_count, 4);
    assert_eq!(report.runtimes.reclaimable_count, 2); // java-8, java-11

    // 5. Clean unused runtimes and assets
    let options = CleanOptions {
        clean_runtimes: true,
        clean_assets: true,
        clean_mods: false,
        clean_libraries: false,
        dry_run: false,
    };
    let summary = clean_unused_cache(&paths, &options).expect("clean should succeed");

    assert_eq!(summary.deleted_runtimes_count, 2);

    // Verify java-17 and java-21 are preserved!
    assert!(paths.runtimes_dir().join("java-17").exists());
    assert!(paths.runtimes_dir().join("java-21").exists());
    assert!(!paths.runtimes_dir().join("java-8").exists());
    assert!(!paths.runtimes_dir().join("java-11").exists());

    // Verify asset indices
    assert!(index_1_20_path.exists());
    assert!(!index_1_16_path.exists());

    // Verify asset object blobs
    assert!(obj_a.exists());
    assert!(!obj_c.exists());
}

#[tokio::test]
async fn test_concurrency_lock_and_running_instance_protection() {
    let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let temp = TempDir::new().unwrap();
    let paths = SzzPaths::custom(temp.path());

    // 1. Lock guard blocks concurrent GC and blocks launch
    assert!(!is_gc_in_progress());
    {
        let _guard = GcLockGuard::acquire().expect("acquire lock guard");
        assert!(is_gc_in_progress());

        // Secondary acquisition must fail with StorageGcInProgress
        let second = GcLockGuard::acquire();
        match second {
            Err(SzzError::StorageGcInProgress) => {}
            other => panic!("expected StorageGcInProgress, got {:?}", other),
        }

        // Launch must fail with StorageGcInProgress
        let auth = DevAuthSession::new("TestUser");
        let reporter = NoopProgressReporter;
        let launch_err = launch_instance(&paths, "test-instance", &auth, &reporter, true)
            .await
            .unwrap_err();
        match launch_err {
            SzzError::StorageGcInProgress => {}
            other => panic!("expected StorageGcInProgress, got {:?}", other),
        }
    }
    // After dropping guard, GC is no longer in progress
    assert!(!is_gc_in_progress());

    // 2. Running instance prevents clean_unused_cache
    let inst_dir = paths.ensure_instance_dirs("running-server").unwrap();
    let meta = dummy_manifest("running-server", "1.20.1", vec![]);
    InstanceMetadata::new("running-server", meta)
        .save_to_dir(&inst_dir)
        .unwrap();

    // Write current process PID to mark it as Running
    let current_pid = std::process::id();
    fs::write(inst_dir.join(".szz_instance.pid"), current_pid.to_string()).unwrap();

    let status = get_instance_status(&paths, "running-server").unwrap();
    assert_eq!(status, InstanceLifecycleStatus::Running);

    // Attempting clean_unused_cache must fail because an instance is running
    let err = clean_unused_cache(&paths, &CleanOptions::default()).unwrap_err();
    match err {
        SzzError::InstanceRunning(id) => assert_eq!(id, "running-server"),
        other => panic!("expected InstanceRunning running error, got {:?}", other),
    }

    // Attempting uninstall_instance must also fail
    let uninstall_err = uninstall_instance(&paths, "running-server").unwrap_err();
    match uninstall_err {
        SzzError::InstanceRunning(id) => assert_eq!(id, "running-server"),
        other => panic!("expected InstanceRunning, got {:?}", other),
    }
}

#[tokio::test]
async fn test_instance_mutation_blocked_during_gc() {
    let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let temp = TempDir::new().unwrap();
    let paths = SzzPaths::custom(temp.path());

    let _guard = GcLockGuard::acquire().expect("acquire GC lock");

    // uninstall_instance must be rejected
    let uninstall_err = uninstall_instance(&paths, "any-server").unwrap_err();
    match uninstall_err {
        SzzError::StorageGcInProgress => {}
        other => panic!("expected StorageGcInProgress, got {:?}", other),
    }

    // install_instance must be rejected
    let manifest = dummy_manifest("any-server", "1.20.1", vec![]);
    let reporter = NoopProgressReporter;
    let install_err = install_instance(&paths, &manifest, &reporter, true)
        .await
        .unwrap_err();
    match install_err {
        SzzError::StorageGcInProgress => {}
        other => panic!("expected StorageGcInProgress, got {:?}", other),
    }
}

#[test]
fn test_corrupted_metadata_fallback_protects_physical_mods() {
    let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let temp = TempDir::new().unwrap();
    let paths = SzzPaths::custom(temp.path());

    // 1. Create a store mod
    fs::create_dir_all(paths.store_dir()).unwrap();
    let temp_file = temp.path().join("source.jar");
    let mod_bytes = b"important-physical-mod-content";
    fs::write(&temp_file, mod_bytes).unwrap();
    let mod_sha1 = szz_core::store::compute_file_hash(&temp_file, szz_core::manifest::HashAlgorithm::Sha1).unwrap();
    let store_file = paths.store_dir().join(format!("{}.jar", mod_sha1));
    fs::copy(&temp_file, &store_file).unwrap();

    // 2. Create instance with CORRUPTED metadata (.szz_instance.json has invalid JSON)
    let inst_dir = paths.ensure_instance_dirs("corrupted-inst").unwrap();
    fs::write(inst_dir.join(".szz_instance.json"), b"INVALID NOT JSON").unwrap();

    // 3. Put hardlink in instance mods directory
    let inst_mod = paths.instance_mods_dir("corrupted-inst").join("mod.jar");
    fs::hard_link(&store_file, &inst_mod).unwrap();

    // 4. Prune should NOT delete the mod because corrupted metadata fallback scans physical mods/ directory
    let summary = prune_orphaned_mods(&paths).expect("prune should succeed despite corrupted metadata");
    assert_eq!(summary.pruned_count, 0);
    assert!(store_file.exists());
    assert!(inst_mod.exists());

    // 5. Clean unused cache should also succeed without error
    let clean_res = clean_unused_cache(&paths, &CleanOptions::default()).expect("clean should succeed");
    assert_eq!(clean_res.deleted_mods_count, 0);
    assert!(store_file.exists());
}

#[test]
fn test_clean_unused_cache_dry_run_simulation() {
    let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let temp = TempDir::new().unwrap();
    let paths = SzzPaths::custom(temp.path());

    // Set up an unused runtime
    let rt_dir = paths.runtimes_dir().join("java-8").join("bin");
    fs::create_dir_all(&rt_dir).unwrap();
    fs::write(rt_dir.join("java"), b"mock-java").unwrap();

    let options = CleanOptions {
        clean_runtimes: true,
        dry_run: true,
        ..Default::default()
    };

    let summary = clean_unused_cache(&paths, &options).expect("dry run clean should succeed");
    assert!(summary.dry_run);
    assert_eq!(summary.deleted_runtimes_count, 1);
    assert!(summary.reclaimed_bytes > 0);

    // Verify java-8 was NOT physically deleted in dry-run
    assert!(paths.runtimes_dir().join("java-8").exists());
}

#[test]
fn test_asset_objects_preserved_when_instances_exist_but_index_unresolved() {
    let _lock = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let temp = TempDir::new().unwrap();
    let paths = SzzPaths::custom(temp.path());

    // Create an instance without asset index json
    let inst_dir = paths.ensure_instance_dirs("server-active").unwrap();
    let meta = dummy_manifest("server-active", "1.20.1", vec![]);
    InstanceMetadata::new("server-active", meta).save_to_dir(&inst_dir).unwrap();

    // Create an asset object blob
    let obj_dir = paths.assets_objects_dir().join("ee");
    fs::create_dir_all(&obj_dir).unwrap();
    let obj_file = obj_dir.join("eeee111122223333");
    fs::write(&obj_file, b"vanilla-asset-bytes").unwrap();

    // Run clean_assets
    let options = CleanOptions {
        clean_assets: true,
        ..Default::default()
    };
    let summary = clean_unused_cache(&paths, &options).expect("clean should succeed");

    // Asset object must NOT be deleted because instances exist and active hashes could not be resolved!
    assert_eq!(summary.deleted_assets_count, 0);
    assert!(obj_file.exists());
}
