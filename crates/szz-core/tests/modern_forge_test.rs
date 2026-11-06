use std::collections::HashMap;
use szz_core::loaders::common::fetch_prism_meta;
use szz_core::loaders::modern_forge::substitute_template_vars;
use szz_core::loaders::natives::is_version_at_least;
use szz_core::paths::SzzPaths;
use tempfile::tempdir;

#[test]
fn test_substitute_template_vars() {
    let mut vars = HashMap::new();
    vars.insert("library_directory", "/home/user/.szz/libraries");
    vars.insert("classpath_separator", ":");
    vars.insert("version_name", "1.20.1");
    vars.insert("natives_directory", "/home/user/.szz/servers/test-srv/natives");

    let raw_arg = "-Djava.library.path=${natives_directory}";
    let expanded = substitute_template_vars(raw_arg, &vars);
    assert_eq!(
        expanded,
        "-Djava.library.path=/home/user/.szz/servers/test-srv/natives"
    );

    let raw_cp = "${library_directory}${classpath_separator}extra.jar";
    let expanded_cp = substitute_template_vars(raw_cp, &vars);
    assert_eq!(expanded_cp, "/home/user/.szz/libraries:extra.jar");
}

#[test]
fn test_client_jar_exclusion_for_mc_117_plus() {
    // Starting with 1.17, client.jar must NOT be in the main classpath for Modern Forge / NeoForge
    assert!(is_version_at_least("1.20.1", "1.17"));
    assert!(is_version_at_least("1.18.2", "1.17"));
    assert!(is_version_at_least("1.17.1", "1.17"));
    assert!(is_version_at_least("1.17", "1.17"));

    // Pre-1.17 versions include client.jar in classpath
    assert!(!is_version_at_least("1.16.5", "1.17"));
    assert!(!is_version_at_least("1.12.2", "1.17"));
    assert!(!is_version_at_least("1.7.10", "1.17"));
}

#[tokio::test]
async fn test_fetch_prism_meta_modern_forge_live() {
    let tmp = tempdir().expect("temp dir");
    let paths = SzzPaths::custom(tmp.path());

    let meta = fetch_prism_meta(&paths, "net.minecraftforge", "47.2.0")
        .await
        .expect("should fetch Forge 47.2.0 meta");

    assert_eq!(
        meta.main_class.as_deref(),
        Some("io.github.zekerzhayard.forgewrapper.installer.Main")
    );
    assert!(!meta.libraries.is_empty());
    assert!(meta
        .libraries
        .iter()
        .any(|lib| lib.name.starts_with("io.github.zekerzhayard:ForgeWrapper")));
}

#[tokio::test]
async fn test_fetch_prism_meta_neoforge_live() {
    let tmp = tempdir().expect("temp dir");
    let paths = SzzPaths::custom(tmp.path());

    let meta = fetch_prism_meta(&paths, "net.neoforged", "20.4.80-beta")
        .await
        .expect("should fetch NeoForge 20.4.80-beta meta");

    assert_eq!(
        meta.main_class.as_deref(),
        Some("io.github.zekerzhayard.forgewrapper.installer.Main")
    );
    assert!(!meta.libraries.is_empty());
    assert!(meta
        .libraries
        .iter()
        .any(|lib| lib.name.starts_with("io.github.zekerzhayard:ForgeWrapper")));
}

#[tokio::test]
async fn test_neoforge_version_prefix_normalization() {
    let tmp = tempdir().expect("temp dir");
    let paths = SzzPaths::custom(tmp.path());

    // Call with leading MC version prefix "1.20.4-20.4.80-beta"
    // which normalizes to "20.4.80-beta"
    let meta = fetch_prism_meta(&paths, "net.neoforged", "1.20.4-20.4.80-beta")
        .await
        .expect("should normalize NeoForge version and fetch meta");

    assert_eq!(
        meta.main_class.as_deref(),
        Some("io.github.zekerzhayard.forgewrapper.installer.Main")
    );

    let cache_file = paths
        .meta_cache_dir()
        .join("net.neoforged")
        .join("20.4.80-beta.json");
    assert!(cache_file.exists(), "cache file must be named with normalized version");
}
