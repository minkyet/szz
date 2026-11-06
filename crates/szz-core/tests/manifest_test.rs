use szz_core::manifest::{HashAlgorithm, LoaderType, ServerManifest};
use szz_core::paths::SzzPaths;
use tempfile::tempdir;

#[test]
fn test_parse_valid_manifest() {
    let json = r#"{
        "serverId": "test-srv",
        "name": "Test Server",
        "serverAddress": "mc.example.com:25565",
        "minecraftVersion": "1.20.1",
        "loader": {
            "type": "fabric",
            "version": "0.15.11"
        },
        "configUrl": null,
        "manifestVersion": 1,
        "mods": [
            {
                "name": "Fabric API",
                "source": "modrinth",
                "fileUrl": "https://cdn.modrinth.com/fabric-api.jar",
                "sha1": "abcdef1234567890abcdef1234567890abcdef12"
            }
        ]
    }"#;

    let manifest = ServerManifest::from_json_str(json).expect("should parse valid manifest");
    assert_eq!(manifest.server_id, "test-srv");
    assert_eq!(manifest.loader.loader_type, LoaderType::Fabric);
    assert_eq!(manifest.required_java_version(), 17);
    assert_eq!(manifest.mods.len(), 1);

    let (hash, algo) = manifest.mods[0].primary_hash().unwrap();
    assert_eq!(algo, HashAlgorithm::Sha1);
    assert_eq!(hash, "abcdef1234567890abcdef1234567890abcdef12");
}

#[test]
fn test_parse_sample_fixture() {
    let manifest_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures")
        .join("sample-manifest.json");

    let manifest = ServerManifest::from_file(&manifest_path).expect("fixture should parse");
    assert_eq!(manifest.server_id, "szz-test-fabric");
    assert_eq!(manifest.minecraft_version, "1.20.1");
    assert_eq!(manifest.required_java_version(), 17);
}

#[test]
fn test_missing_required_fields_fails() {
    let json = r#"{
        "serverId": "",
        "name": "Invalid Server",
        "serverAddress": "",
        "minecraftVersion": "1.20.1",
        "loader": { "type": "fabric", "version": "0.15" },
        "manifestVersion": 1,
        "mods": []
    }"#;

    let err = ServerManifest::from_json_str(json).unwrap_err();
    assert!(err.to_string().contains("serverId must not be empty"));
}

#[test]
fn test_paths_ensure_dirs() {
    let tmp = tempdir().unwrap();
    let paths = SzzPaths::custom(tmp.path().join(".szz"));

    assert!(!paths.store_dir().exists());
    paths.ensure_dirs().expect("dirs should be created");
    assert!(paths.store_dir().exists());
    assert!(paths.runtimes_dir().exists());
    assert!(paths.instances_dir().exists());

    let instance = paths.ensure_instance_dirs("srv-1").expect("instance dirs created");
    assert!(instance.join("mods").exists());
}

#[test]
fn test_required_java_version_mapping() {
    fn make_manifest(mc_version: &str) -> ServerManifest {
        let json = format!(
            r#"{{
                "serverId": "test",
                "name": "Test",
                "serverAddress": "127.0.0.1:25565",
                "minecraftVersion": "{}",
                "loader": {{ "type": "fabric", "version": "0.15" }},
                "manifestVersion": 1,
                "mods": []
            }}"#,
            mc_version
        );
        ServerManifest::from_json_str(&json).unwrap()
    }

    assert_eq!(make_manifest("1.7.10").required_java_version(), 8);
    assert_eq!(make_manifest("1.8.9").required_java_version(), 8);
    assert_eq!(make_manifest("1.10.2").required_java_version(), 8);
    assert_eq!(make_manifest("1.11.2").required_java_version(), 8);
    assert_eq!(make_manifest("1.12.2").required_java_version(), 8);
    assert_eq!(make_manifest("1.13.2").required_java_version(), 8);
    assert_eq!(make_manifest("1.14.4").required_java_version(), 8);
    assert_eq!(make_manifest("1.15.2").required_java_version(), 8);
    assert_eq!(make_manifest("1.16.5").required_java_version(), 8);
    assert_eq!(make_manifest("1.17.1").required_java_version(), 16);
    assert_eq!(make_manifest("1.18.2").required_java_version(), 17);
    assert_eq!(make_manifest("1.20.1").required_java_version(), 17);
    assert_eq!(make_manifest("1.20.4").required_java_version(), 17);
    assert_eq!(make_manifest("1.20.5").required_java_version(), 21);
    assert_eq!(make_manifest("1.20.6").required_java_version(), 21);
    assert_eq!(make_manifest("1.21.1").required_java_version(), 21);
    assert_eq!(make_manifest("26.1").required_java_version(), 25);
    assert_eq!(make_manifest("26.3").required_java_version(), 25);
}

#[test]
fn test_config_spec_deserialization() {
    // 1. null configUrl
    let json_null = r#"{
        "serverId": "test-srv",
        "name": "Test",
        "serverAddress": "127.0.0.1:25565",
        "minecraftVersion": "1.20.1",
        "loader": { "type": "fabric", "version": "0.15" },
        "configUrl": null,
        "manifestVersion": 1,
        "mods": []
    }"#;
    let manifest = ServerManifest::from_json_str(json_null).unwrap();
    assert!(manifest.config.is_none());
    assert_eq!(manifest.config_url(), None);

    // 2. string configUrl (backwards compatible)
    let json_str = r#"{
        "serverId": "test-srv",
        "name": "Test",
        "serverAddress": "127.0.0.1:25565",
        "minecraftVersion": "1.20.1",
        "loader": { "type": "fabric", "version": "0.15" },
        "configUrl": "https://example.com/configs/v1.zip",
        "manifestVersion": 1,
        "mods": []
    }"#;
    let manifest = ServerManifest::from_json_str(json_str).unwrap();
    assert_eq!(
        manifest.config,
        Some(szz_core::manifest::ConfigSpec {
            url: "https://example.com/configs/v1.zip".to_string(),
            hash: None,
            version: None,
        })
    );
    assert_eq!(manifest.config_url(), Some("https://example.com/configs/v1.zip"));

    // 3. full object config
    let json_obj = r#"{
        "serverId": "test-srv",
        "name": "Test",
        "serverAddress": "127.0.0.1:25565",
        "minecraftVersion": "1.20.1",
        "loader": { "type": "fabric", "version": "0.15" },
        "config": {
            "url": "https://example.com/configs/v2.zip",
            "hash": "sha1:3e9cdd3e2f827ca9a259df9eb8e31949437b6bd4",
            "version": "2.1.0"
        },
        "manifestVersion": 1,
        "mods": []
    }"#;
    let manifest = ServerManifest::from_json_str(json_obj).unwrap();
    let cfg = manifest.config.unwrap();
    assert_eq!(cfg.url, "https://example.com/configs/v2.zip");
    assert_eq!(cfg.hash.as_deref(), Some("sha1:3e9cdd3e2f827ca9a259df9eb8e31949437b6bd4"));
    assert_eq!(cfg.version.as_deref(), Some("2.1.0"));
}

#[test]
fn test_server_id_validation() {
    fn validate_id(id: &str) -> bool {
        let json = format!(
            r#"{{
                "serverId": "{}",
                "name": "Test",
                "serverAddress": "127.0.0.1:25565",
                "minecraftVersion": "1.20.1",
                "loader": {{ "type": "fabric", "version": "0.15" }},
                "manifestVersion": 1,
                "mods": []
            }}"#,
            id
        );
        ServerManifest::from_json_str(&json).is_ok()
    }

    assert!(validate_id("valid-server-id"));
    assert!(validate_id("server_123"));
    assert!(validate_id("AlphaBeta-99_x"));

    // Invalid IDs
    assert!(!validate_id(""));
    assert!(!validate_id("../evil"));
    assert!(!validate_id("server/with/slash"));
    assert!(!validate_id("server with spaces"));
    assert!(!validate_id("server!@#$%"));
}

#[test]
fn test_all_loader_types_deserialization() {
    let loaders = [
        ("fabric", LoaderType::Fabric),
        ("Fabric", LoaderType::Fabric),
        ("forge", LoaderType::Forge),
        ("Forge", LoaderType::Forge),
        ("neoforge", LoaderType::NeoForge),
        ("neoforged", LoaderType::NeoForge),
        ("NeoForge", LoaderType::NeoForge),
        ("NeoForged", LoaderType::NeoForge),
        ("liteloader", LoaderType::LiteLoader),
        ("LiteLoader", LoaderType::LiteLoader),
        ("vanilla", LoaderType::Vanilla),
        ("Vanilla", LoaderType::Vanilla),
    ];

    for (name, expected_type) in loaders {
        let json = format!(
            r#"{{
                "serverId": "test-srv",
                "name": "Test",
                "serverAddress": "127.0.0.1:25565",
                "minecraftVersion": "1.20.1",
                "loader": {{ "type": "{}", "version": "1.0" }},
                "manifestVersion": 1,
                "mods": []
            }}"#,
            name
        );
        let manifest = ServerManifest::from_json_str(&json).expect("should parse manifest");
        assert_eq!(manifest.loader.loader_type, expected_type);
    }
}

#[test]
fn test_manifest_is_verified() {
    let verified_json = r#"{
        "serverId": "verified-srv",
        "name": "Verified Server",
        "serverAddress": "127.0.0.1:25565",
        "minecraftVersion": "1.20.1",
        "loader": { "type": "fabric", "version": "0.16.10" },
        "manifestVersion": 1,
        "mods": [
            {
                "name": "Fabric API",
                "source": "modrinth",
                "fileUrl": "https://cdn.modrinth.com/data/fabric.jar",
                "sha1": "3e9cdd3e2f827ca9a259df9eb8e31949437b6bd4"
            }
        ]
    }"#;
    let verified = ServerManifest::from_json_str(verified_json).unwrap();
    assert!(verified.is_verified());

    let unverified_json = r#"{
        "serverId": "custom-srv",
        "name": "Custom Mod Server",
        "serverAddress": "127.0.0.1:25565",
        "minecraftVersion": "1.20.1",
        "loader": { "type": "fabric", "version": "0.16.10" },
        "manifestVersion": 1,
        "mods": [
            {
                "name": "Custom Server Core",
                "source": "custom",
                "fileUrl": "https://example.com/custom.jar",
                "sha1": "3e9cdd3e2f827ca9a259df9eb8e31949437b6bd4"
            }
        ]
    }"#;
    let unverified = ServerManifest::from_json_str(unverified_json).unwrap();
    assert!(!unverified.is_verified());
}

#[test]
fn test_manifest_rejects_invalid_mod_and_config_hashes() {
    // Malicious or malformed sha1 (path traversal or wrong length)
    let bad_sha1_json = r#"{
        "serverId": "test-srv",
        "name": "Test Server",
        "serverAddress": "127.0.0.1:25565",
        "minecraftVersion": "1.20.1",
        "loader": { "type": "fabric", "version": "0.16.10" },
        "manifestVersion": 1,
        "mods": [
            {
                "name": "Evil Mod",
                "source": "custom",
                "fileUrl": "https://example.com/evil.jar",
                "sha1": "../../escape-hash"
            }
        ]
    }"#;
    assert!(ServerManifest::from_json_str(bad_sha1_json).is_err());

    // Invalid sha1 with non-hex character
    let non_hex_sha1 = r#"{
        "serverId": "test-srv",
        "name": "Test Server",
        "serverAddress": "127.0.0.1:25565",
        "minecraftVersion": "1.20.1",
        "loader": { "type": "fabric", "version": "0.16.10" },
        "manifestVersion": 1,
        "mods": [
            {
                "name": "Non Hex Mod",
                "source": "custom",
                "fileUrl": "https://example.com/mod.jar",
                "sha1": "3e9cdd3e2f827ca9a259df9eb8e31949437b6bzz"
            }
        ]
    }"#;
    assert!(ServerManifest::from_json_str(non_hex_sha1).is_err());

    // Invalid config hash
    let bad_config_hash = r#"{
        "serverId": "test-srv",
        "name": "Test Server",
        "serverAddress": "127.0.0.1:25565",
        "minecraftVersion": "1.20.1",
        "loader": { "type": "fabric", "version": "0.16.10" },
        "manifestVersion": 1,
        "config": {
            "url": "https://example.com/config.zip",
            "hash": "sha1:../../etc/passwd"
        },
        "mods": []
    }"#;
    assert!(ServerManifest::from_json_str(bad_config_hash).is_err());
}

