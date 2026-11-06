use szz_core::fabric::{fetch_fabric_metadata, maven_coordinate_to_path};

#[test]
fn test_maven_coordinate_parsing() {
    let path = maven_coordinate_to_path("org.ow2.asm:asm:9.6").unwrap();
    assert_eq!(path, "org/ow2/asm/asm/9.6/asm-9.6.jar");

    let complex = maven_coordinate_to_path("net.fabricmc:sponge-mixin:0.13.3+mixin.0.8.5").unwrap();
    assert_eq!(
        complex,
        "net/fabricmc/sponge-mixin/0.13.3+mixin.0.8.5/sponge-mixin-0.13.3+mixin.0.8.5.jar"
    );

    assert!(maven_coordinate_to_path("invalid-id").is_none());
}

#[tokio::test]
async fn test_fetch_fabric_meta_live() {
    let meta = fetch_fabric_metadata("1.20.1", "0.15.11")
        .await
        .expect("should fetch fabric metadata");

    assert_eq!(meta.loader.version, "0.15.11");
    assert_eq!(
        meta.launcher_meta.main_class.client,
        "net.fabricmc.loader.impl.launch.knot.KnotClient"
    );
    assert!(!meta.launcher_meta.libraries.common.is_empty());
}
