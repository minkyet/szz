use szz_core::loaders::common::{fetch_prism_meta, PrismComponentMeta};
use szz_core::paths::SzzPaths;
use tempfile::tempdir;

#[test]
fn test_parse_prism_forge_meta_structure() {
    let sample_json = r#"{
        "formatVersion": 1,
        "name": "Forge",
        "uid": "net.minecraftforge",
        "version": "14.23.5.2860",
        "mainClass": "net.minecraft.launchwrapper.Launch",
        "+tweakers": [
            "net.minecraftforge.fml.common.launcher.FMLTweaker"
        ],
        "+gameArgs": [
            "--demo"
        ],
        "+jvmArgs": [
            "-Dfml.ignoreInvalidMinecraftCertificates=true"
        ],
        "libraries": [
            {
                "name": "net.minecraftforge:forge:1.12.2-14.23.5.2860:universal",
                "downloads": {
                    "artifact": {
                        "path": "net/minecraftforge/forge/1.12.2-14.23.5.2860/forge-1.12.2-14.23.5.2860-universal.jar",
                        "sha1": "029250575d3aa2cf80b56dffb66238a1eeaea2ac",
                        "size": 4466148,
                        "url": "https://maven.minecraftforge.net/net/minecraftforge/forge/1.12.2-14.23.5.2860/forge-1.12.2-14.23.5.2860-universal.jar"
                    }
                }
            },
            {
                "name": "net.minecraft:launchwrapper:1.12",
                "downloads": {
                    "artifact": {
                        "path": "net/minecraft/launchwrapper/1.12/launchwrapper-1.12.jar",
                        "sha1": "111e7bea9c968cdb3d06ef4632bf7ff0824d0f36",
                        "size": 32999,
                        "url": "https://libraries.minecraft.net/net/minecraft/launchwrapper/1.12/launchwrapper-1.12.jar"
                    }
                }
            }
        ]
    }"#;

    let meta: PrismComponentMeta =
        serde_json::from_str(sample_json).expect("should parse PrismMeta JSON");

    assert_eq!(
        meta.main_class.as_deref(),
        Some("net.minecraft.launchwrapper.Launch")
    );
    assert_eq!(
        meta.tweakers,
        vec!["net.minecraftforge.fml.common.launcher.FMLTweaker"]
    );
    assert_eq!(meta.game_args, vec!["--demo"]);
    assert_eq!(
        meta.jvm_args,
        vec!["-Dfml.ignoreInvalidMinecraftCertificates=true"]
    );
    assert_eq!(meta.libraries.len(), 2);
    assert_eq!(
        meta.libraries[0].name,
        "net.minecraftforge:forge:1.12.2-14.23.5.2860:universal"
    );
    assert_eq!(
        meta.libraries[0]
            .downloads
            .as_ref()
            .unwrap()
            .artifact
            .as_ref()
            .unwrap()
            .path,
        "net/minecraftforge/forge/1.12.2-14.23.5.2860/forge-1.12.2-14.23.5.2860-universal.jar"
    );
}

#[tokio::test]
async fn test_fetch_prism_meta_forge_live_and_cache() {
    let tmp = tempdir().expect("temp dir");
    let paths = SzzPaths::custom(tmp.path());

    // 1. Fetch live from PrismMeta
    let meta = fetch_prism_meta(&paths, "net.minecraftforge", "14.23.5.2860")
        .await
        .expect("should fetch live Forge meta from PrismMeta");

    assert_eq!(
        meta.main_class.as_deref(),
        Some("net.minecraft.launchwrapper.Launch")
    );
    assert!(meta
        .tweakers
        .contains(&"net.minecraftforge.fml.common.launcher.FMLTweaker".to_string()));
    assert!(!meta.libraries.is_empty());

    // 2. Verify cache file was written to ~/.szz/cache/meta/net.minecraftforge/14.23.5.2860.json
    let cache_file = paths
        .meta_cache_dir()
        .join("net.minecraftforge")
        .join("14.23.5.2860.json");
    assert!(cache_file.exists(), "cache file must be created on disk");

    // 3. Second call with version prefix "1.12.2-14.23.5.2860" should read from cache
    let cached_meta = fetch_prism_meta(&paths, "net.minecraftforge", "1.12.2-14.23.5.2860")
        .await
        .expect("should resolve normalized version from cache");
    assert_eq!(
        cached_meta.main_class.as_deref(),
        Some("net.minecraft.launchwrapper.Launch")
    );
}
