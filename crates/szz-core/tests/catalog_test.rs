use szz_core::catalog::{
    fetch_remote_catalog, load_cached_catalog, load_cache_meta, save_cached_catalog,
    validate_url_scheme, RemoteCatalog,
};
use szz_core::config::catalog_url;
use szz_core::paths::SzzPaths;
use tempfile::TempDir;

#[test]
fn test_validate_url_scheme() {
    assert!(validate_url_scheme("https://catalog.szz-mc.net/catalog.json").is_ok());
    assert!(validate_url_scheme("http://localhost:8080/servers").is_ok());

    assert!(validate_url_scheme("file:///etc/passwd").is_err());
    assert!(validate_url_scheme("javascript:alert(1)").is_err());
    assert!(validate_url_scheme("data:text/html,<html>").is_err());
    assert!(validate_url_scheme("ftp://files.example.com/catalog.json").is_err());
    assert!(validate_url_scheme("not-a-valid-url").is_err());
}

#[test]
fn test_catalog_serialization_and_deserialization() {
    let json = r#"{
        "version": 1,
        "updatedAt": "2026-10-27T12:00:00Z",
        "servers": [
            {
                "id": "szz-test-fabric",
                "name": "SZZ 테스트 서버",
                "tagline": "테스트 서버 설명",
                "manifestUrl": "https://cdn.example.com/servers/szz-test-fabric/manifest.json",
                "latestManifestVersion": 3,
                "verified": true
            }
        ]
    }"#;

    let catalog: RemoteCatalog = serde_json::from_str(json).expect("should parse catalog json");
    assert_eq!(catalog.version, 1);
    assert_eq!(catalog.servers.len(), 1);
    let srv = &catalog.servers[0];
    assert_eq!(srv.id, "szz-test-fabric");
    assert_eq!(srv.name, "SZZ 테스트 서버");
    assert_eq!(srv.latest_manifest_version, 3);
    assert!(srv.verified);

    let serialized = serde_json::to_string(&catalog).unwrap();
    assert!(serialized.contains("szz-test-fabric"));
    assert!(!serialized.contains("bannerUrl"));
    assert!(!serialized.contains("recommendedMemoryMb"));
}

#[test]
fn test_cache_lifecycle_and_atomic_save() {
    let temp = TempDir::new().unwrap();
    let paths = SzzPaths::custom(temp.path());

    let json = r#"{
        "version": 1,
        "servers": [
            {
                "id": "demo-server",
                "name": "Demo",
                "tagline": "Demo Tagline",
                "manifestUrl": "https://cdn.example.com/manifest.json",
                "latestManifestVersion": 1,
                "verified": false
            }
        ]
    }"#;

    // Initially no cache
    assert!(load_cached_catalog(&paths.catalog_cache_file()).is_none());
    assert!(load_cache_meta(&paths.catalog_meta_file()).is_none());

    // Save with ETag
    save_cached_catalog(
        &paths,
        json,
        Some("\"etag-hash-12345\"".to_string()),
        Some("Wed, 21 Oct 2026 07:28:00 GMT".to_string()),
    )
    .expect("save should succeed");

    // Load back
    let cached = load_cached_catalog(&paths.catalog_cache_file()).expect("cache should exist");
    assert_eq!(cached.servers.len(), 1);
    assert_eq!(cached.servers[0].id, "demo-server");

    let meta = load_cache_meta(&paths.catalog_meta_file()).expect("meta should exist");
    assert_eq!(meta.etag, Some("\"etag-hash-12345\"".to_string()));
    assert_eq!(meta.last_modified, Some("Wed, 21 Oct 2026 07:28:00 GMT".to_string()));
    assert!(meta.fetched_at > 0);
}

#[tokio::test]
async fn test_fetch_remote_catalog_offline_fallback() {
    let temp = TempDir::new().unwrap();
    let paths = SzzPaths::custom(temp.path());
    let client = reqwest::Client::new();

    // 1. Unreachable URL without cache returns error
    let unreachable_url = "http://127.0.0.1:54321/catalog.json";
    let res = fetch_remote_catalog(&client, &paths, unreachable_url).await;
    assert!(res.is_err(), "should fail without local cache");

    // 2. Pre-seed local cache
    let json = r#"{
        "version": 1,
        "servers": [
            {
                "id": "cached-srv",
                "name": "Cached Server",
                "tagline": "Offline Tagline",
                "manifestUrl": "https://example.com/manifest.json",
                "latestManifestVersion": 2,
                "verified": true
            }
        ]
    }"#;
    save_cached_catalog(&paths, json, None, None).unwrap();

    // 3. Unreachable URL with existing cache falls back gracefully
    let res_fallback = fetch_remote_catalog(&client, &paths, unreachable_url)
        .await
        .expect("should fall back to local cache");

    assert!(res_fallback.is_offline);
    assert!(res_fallback.from_cache);
    assert_eq!(res_fallback.catalog.servers.len(), 1);
    assert_eq!(res_fallback.catalog.servers[0].id, "cached-srv");
}

#[test]
fn test_config_catalog_url_env_resolution() {
    // Runtime override takes highest precedence
    std::env::set_var("SZZ_CATALOG_URL", "https://custom.cdn.net/custom-catalog.json");
    assert_eq!(catalog_url(), "https://custom.cdn.net/custom-catalog.json");
    std::env::remove_var("SZZ_CATALOG_URL");

    // Fallback returns either runtime .env or DEFAULT_CATALOG_URL
    let fallback = catalog_url();
    assert!(!fallback.is_empty());
}
