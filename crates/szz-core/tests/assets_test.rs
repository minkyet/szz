use sha1::Digest;
use std::collections::HashMap;
use szz_core::assets::{
    validate_asset_index_id, validate_hash, AssetIndexFile, AssetObject, AssetSyncResult,
};
use szz_core::paths::SzzPaths;
use szz_core::NoopProgressReporter;
use tempfile::tempdir;

#[test]
fn test_assets_path_helpers() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().join(".szz");
    let paths = SzzPaths::custom(root.clone());

    paths.ensure_dirs().unwrap();

    assert_eq!(paths.assets_dir(), root.join("assets"));
    assert_eq!(paths.assets_indexes_dir(), root.join("assets").join("indexes"));
    assert_eq!(paths.assets_objects_dir(), root.join("assets").join("objects"));

    assert!(paths.assets_indexes_dir().is_dir());
    assert!(paths.assets_objects_dir().is_dir());

    let index_file = paths.asset_index_path("1.20");
    assert_eq!(
        index_file,
        root.join("assets").join("indexes").join("1.20.json")
    );

    let hash = "b2c1d3e4f50123456789abcdef0123456789abcd";
    let object_file = paths.asset_object_path(hash);
    assert_eq!(
        object_file,
        root.join("assets")
            .join("objects")
            .join("b2")
            .join("b2c1d3e4f50123456789abcdef0123456789abcd")
    );
}

#[test]
fn test_assets_security_guards() {
    // Valid 40-char hex hashes
    assert!(validate_hash("b2c1d3e4f50123456789abcdef0123456789abcd").is_ok());
    assert!(validate_hash("B2C1D3E4F50123456789ABCDEF0123456789ABCD").is_ok());
    assert!(validate_hash("0000000000000000000000000000000000000000").is_ok());

    // Invalid hashes
    assert!(validate_hash("").is_err());
    assert!(validate_hash("short").is_err());
    assert!(validate_hash("b2c1d3e4f50123456789abcdef0123456789abc").is_err()); // 39 chars
    assert!(validate_hash("b2c1d3e4f50123456789abcdef0123456789abcde").is_err()); // 41 chars
    assert!(validate_hash("../b2c1d3e4f50123456789abcdef0123456789").is_err());
    assert!(validate_hash("g2c1d3e4f50123456789abcdef0123456789abcd").is_err()); // 'g' is non-hex

    // Valid asset index IDs
    assert!(validate_asset_index_id("1.20").is_ok());
    assert!(validate_asset_index_id("1.20.1").is_ok());
    assert!(validate_asset_index_id("pre-1.12").is_ok());
    assert!(validate_asset_index_id("legacy").is_ok());

    // Invalid asset index IDs
    assert!(validate_asset_index_id("").is_err());
    assert!(validate_asset_index_id("../1.20").is_err());
    assert!(validate_asset_index_id("1.20/..").is_err());
    assert!(validate_asset_index_id("dir/1.20").is_err());
    assert!(validate_asset_index_id("dir\\1.20").is_err());
}

#[test]
fn test_assets_index_parsing() {
    let json_data = r#"{
        "objects": {
            "icons/icon_16x16.png": {
                "hash": "b2c1d3e4f50123456789abcdef0123456789abcd",
                "size": 1024
            },
            "minecraft/sounds/ambient/cave/cave1.ogg": {
                "hash": "a1b2c3d4e5f60123456789abcdef0123456789ab",
                "size": 2048
            }
        }
    }"#;

    let parsed: AssetIndexFile = serde_json::from_str(json_data).expect("Must parse asset index");
    assert_eq!(parsed.objects.len(), 2);

    let icon = parsed.objects.get("icons/icon_16x16.png").unwrap();
    assert_eq!(icon.hash, "b2c1d3e4f50123456789abcdef0123456789abcd");
    assert_eq!(icon.size, 1024);

    let sound = parsed
        .objects
        .get("minecraft/sounds/ambient/cave/cave1.ogg")
        .unwrap();
    assert_eq!(sound.hash, "a1b2c3d4e5f60123456789abcdef0123456789ab");
    assert_eq!(sound.size, 2048);
}

#[tokio::test]
async fn test_assets_dry_run_noop() {
    let tmp = tempdir().unwrap();
    let paths = SzzPaths::custom(tmp.path().join(".szz"));
    let reporter = NoopProgressReporter;

    // 1. Without cached version file: returns mc_version
    let res = szz_core::assets::sync_mojang_assets(&paths, "1.20.1", &reporter, true)
        .await
        .unwrap();
    assert_eq!(
        res,
        AssetSyncResult {
            asset_index_id: "1.20.1".to_string(),
            total_objects: 0,
            downloaded_objects: 0,
        }
    );

    // 2. With cached version file: resolves asset_index_id from version json
    let ver_dir = paths.version_dir("1.20.1");
    std::fs::create_dir_all(&ver_dir).unwrap();
    let version_json = r#"{
        "id": "1.20.1",
        "assetIndex": {
            "id": "1.20",
            "sha1": "0000000000000000000000000000000000000000",
            "size": 100,
            "url": "https://example.com/asset_index.json"
        }
    }"#;
    std::fs::write(ver_dir.join("1.20.1.json"), version_json).unwrap();

    let res = szz_core::assets::sync_mojang_assets(&paths, "1.20.1", &reporter, true)
        .await
        .unwrap();
    assert_eq!(
        res,
        AssetSyncResult {
            asset_index_id: "1.20".to_string(),
            total_objects: 0,
            downloaded_objects: 0,
        }
    );
}

#[tokio::test]
async fn test_assets_cache_hit_noop() {
    let tmp = tempdir().unwrap();
    let paths = SzzPaths::custom(tmp.path().join(".szz"));
    paths.ensure_dirs().unwrap();
    let reporter = NoopProgressReporter;

    // Object 1: "hello world" -> size 11, sha1: 2aae6c35c94fcfb415dbe95f408b9ce91ee846ed
    let content1 = b"hello world";
    let hash1 = hex::encode(sha1::Sha1::digest(content1));
    let size1 = content1.len() as u64;

    // Object 2: "minecraft asset test" -> size 20, sha1
    let content2 = b"minecraft asset test";
    let hash2 = hex::encode(sha1::Sha1::digest(content2));
    let size2 = content2.len() as u64;

    // Place objects on disk in paths.asset_object_path
    let dest1 = paths.asset_object_path(&hash1);
    std::fs::create_dir_all(dest1.parent().unwrap()).unwrap();
    std::fs::write(&dest1, content1).unwrap();

    let dest2 = paths.asset_object_path(&hash2);
    std::fs::create_dir_all(dest2.parent().unwrap()).unwrap();
    std::fs::write(&dest2, content2).unwrap();

    // Create asset index JSON
    let mut objects = HashMap::new();
    objects.insert(
        "lang/ko_kr.json".to_string(),
        AssetObject {
            hash: hash1.clone(),
            size: size1,
        },
    );
    objects.insert(
        "sounds/test.ogg".to_string(),
        AssetObject {
            hash: hash2.clone(),
            size: size2,
        },
    );
    let index_file_content = AssetIndexFile { objects };
    let index_bytes = serde_json::to_vec(&index_file_content).unwrap();
    let index_sha1 = hex::encode(sha1::Sha1::digest(&index_bytes));
    let index_size = index_bytes.len();

    // Place asset index in paths.asset_index_path("1.20")
    let index_path = paths.asset_index_path("1.20");
    std::fs::create_dir_all(index_path.parent().unwrap()).unwrap();
    std::fs::write(&index_path, &index_bytes).unwrap();

    // Place Mojang version JSON in paths.version_dir("1.20.1")/1.20.1.json
    let ver_dir = paths.version_dir("1.20.1");
    std::fs::create_dir_all(&ver_dir).unwrap();
    let version_json = format!(
        r#"{{
            "id": "1.20.1",
            "assetIndex": {{
                "id": "1.20",
                "sha1": "{}",
                "size": {},
                "url": "https://example.com/asset_index.json"
            }}
        }}"#,
        index_sha1, index_size
    );
    std::fs::write(ver_dir.join("1.20.1.json"), version_json).unwrap();

    // Execute sync_mojang_assets with dry_run = false (0ms No-op cache hit)
    let res = szz_core::assets::sync_mojang_assets(&paths, "1.20.1", &reporter, false)
        .await
        .unwrap();

    assert_eq!(
        res,
        AssetSyncResult {
            asset_index_id: "1.20".to_string(),
            total_objects: 2,
            downloaded_objects: 0,
        }
    );
}

#[tokio::test]
async fn test_assets_fallback_when_asset_index_none() {
    let tmp = tempdir().unwrap();
    let paths = SzzPaths::custom(tmp.path().join(".szz"));
    paths.ensure_dirs().unwrap();
    let reporter = NoopProgressReporter;

    let ver_dir = paths.version_dir("1.20.1");
    std::fs::create_dir_all(&ver_dir).unwrap();
    let version_json = r#"{
        "id": "1.20.1",
        "assetIndex": null
    }"#;
    std::fs::write(ver_dir.join("1.20.1.json"), version_json).unwrap();

    let res = szz_core::assets::sync_mojang_assets(&paths, "1.20.1", &reporter, false)
        .await
        .unwrap();

    assert_eq!(
        res,
        AssetSyncResult {
            asset_index_id: "1.20.1".to_string(),
            total_objects: 0,
            downloaded_objects: 0,
        }
    );
}

#[test]
fn test_szz_home_override() {
    let tmp = tempdir().unwrap();
    let custom_dir = tmp.path().join("custom_szz_home");

    // Set SZZ_HOME
    std::env::set_var("SZZ_HOME", &custom_dir);
    let paths = SzzPaths::new().unwrap();
    assert_eq!(paths.root, custom_dir);

    // Empty or whitespace-only SZZ_HOME should fallback
    std::env::set_var("SZZ_HOME", "   ");
    let paths_fallback = SzzPaths::new().unwrap();
    assert_ne!(paths_fallback.root, custom_dir);

    // Clean up env
    std::env::remove_var("SZZ_HOME");
}

#[tokio::test]
async fn test_assets_duplicate_hash_deduplication() {
    let tmp = tempdir().unwrap();
    let paths = SzzPaths::custom(tmp.path().join(".szz"));
    paths.ensure_dirs().unwrap();
    let reporter = NoopProgressReporter;

    // Both objects share the exact same hash
    let content = b"shared content across two asset keys";
    let hash = hex::encode(sha1::Sha1::digest(content));
    let size = content.len() as u64;

    let dest = paths.asset_object_path(&hash);
    std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
    std::fs::write(&dest, content).unwrap();

    let mut objects = HashMap::new();
    objects.insert("key1.png".to_string(), AssetObject { hash: hash.clone(), size });
    objects.insert("key2.png".to_string(), AssetObject { hash: hash.clone(), size });
    let index_file_content = AssetIndexFile { objects };
    let index_bytes = serde_json::to_vec(&index_file_content).unwrap();
    let index_sha1 = hex::encode(sha1::Sha1::digest(&index_bytes));
    let index_size = index_bytes.len();

    let index_path = paths.asset_index_path("1.20");
    std::fs::create_dir_all(index_path.parent().unwrap()).unwrap();
    std::fs::write(&index_path, &index_bytes).unwrap();

    let ver_dir = paths.version_dir("1.20.1");
    std::fs::create_dir_all(&ver_dir).unwrap();
    let version_json = format!(
        r#"{{
            "id": "1.20.1",
            "assetIndex": {{
                "id": "1.20",
                "sha1": "{}",
                "size": {},
                "url": "https://example.com/asset_index.json"
            }}
        }}"#,
        index_sha1, index_size
    );
    std::fs::write(ver_dir.join("1.20.1.json"), version_json).unwrap();

    let res = szz_core::assets::sync_mojang_assets(&paths, "1.20.1", &reporter, false)
        .await
        .unwrap();

    assert_eq!(
        res,
        AssetSyncResult {
            asset_index_id: "1.20".to_string(),
            total_objects: 2,
            downloaded_objects: 0,
        }
    );
}

