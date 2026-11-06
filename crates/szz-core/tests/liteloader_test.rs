use szz_core::loaders::common::{fetch_prism_meta, PrismComponentMeta};
use szz_core::manifest::{LoaderType, ServerManifest};
use szz_core::paths::SzzPaths;
use tempfile::tempdir;

#[test]
fn test_parse_prism_liteloader_meta_structure() {
    let sample_json = r#"{
        "formatVersion": 1,
        "name": "LiteLoader",
        "uid": "com.mumfrey.liteloader",
        "version": "1.12.2-SNAPSHOT",
        "mainClass": "net.minecraft.launchwrapper.Launch",
        "+tweakers": [
            "com.mumfrey.liteloader.launch.LiteLoaderTweaker"
        ],
        "libraries": [
            {
                "name": "net.minecraft:launchwrapper:1.12"
            },
            {
                "name": "org.ow2.asm:asm-all:5.2",
                "url": "http://repo.liteloader.com/"
            },
            {
                "name": "com.mumfrey:liteloader:1.12.2-SNAPSHOT",
                "url": "http://dl.liteloader.com/versions/"
            }
        ]
    }"#;

    let meta: PrismComponentMeta =
        serde_json::from_str(sample_json).expect("should parse LiteLoader JSON");

    assert_eq!(
        meta.main_class.as_deref(),
        Some("net.minecraft.launchwrapper.Launch")
    );
    assert_eq!(
        meta.tweakers,
        vec!["com.mumfrey.liteloader.launch.LiteLoaderTweaker"]
    );
    assert_eq!(meta.libraries.len(), 3);
    assert_eq!(
        meta.libraries[2].name,
        "com.mumfrey:liteloader:1.12.2-SNAPSHOT"
    );
    assert_eq!(
        meta.libraries[2].url.as_deref(),
        Some("http://dl.liteloader.com/versions/")
    );
}

#[tokio::test]
async fn test_fetch_prism_meta_liteloader_live() {
    let tmp = tempdir().expect("temp dir");
    let paths = SzzPaths::custom(tmp.path());

    let meta = fetch_prism_meta(&paths, "com.mumfrey.liteloader", "1.12.2-SNAPSHOT")
        .await
        .expect("should fetch LiteLoader 1.12.2-SNAPSHOT meta");

    assert_eq!(
        meta.main_class.as_deref(),
        Some("net.minecraft.launchwrapper.Launch")
    );
    assert!(meta
        .tweakers
        .contains(&"com.mumfrey.liteloader.launch.LiteLoaderTweaker".to_string()));
    assert!(!meta.libraries.is_empty());
}

#[test]
fn test_liteloader_manifest_deserialization() {
    let json = r#"{
        "manifestVersion": 1,
        "serverId": "liteloader-test",
        "name": "LiteLoader Server",
        "serverAddress": "play.example.com",
        "minecraftVersion": "1.12.2",
        "loader": {
            "type": "liteloader",
            "version": "1.12.2-SNAPSHOT"
        },
        "mods": []
    }"#;

    let manifest: ServerManifest = serde_json::from_str(json).expect("should parse manifest");
    assert_eq!(manifest.loader.loader_type, LoaderType::LiteLoader);
    assert_eq!(manifest.loader.version, "1.12.2-SNAPSHOT");
}
