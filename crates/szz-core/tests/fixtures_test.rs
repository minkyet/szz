use std::path::PathBuf;
use szz_core::auth::DevAuthSession;
use szz_core::launcher::{build_command, format_command_line, launch_server_manifest, LaunchConfig};
use szz_core::manifest::{HashAlgorithm, LoaderType, ServerManifest};
use szz_core::paths::SzzPaths;
use szz_core::progress::NoopProgressReporter;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures")
        .join("servers")
}

#[test]
fn test_cobblemon_manifest_validation() {
    let manifest_path = fixtures_dir().join("cobblemon-1.21.1.json");
    let manifest = ServerManifest::from_file(&manifest_path)
        .expect("cobblemon fixture should parse from file");

    manifest.validate().expect("cobblemon fixture must be valid");
    assert_eq!(manifest.server_id, "cobblemon-1-21-1");
    assert_eq!(manifest.name, "Cobblemon Official (Fabric 1.21.1)");
    assert_eq!(manifest.server_address, "cobblemon.example.com");
    assert_eq!(manifest.minecraft_version, "1.21.1");
    assert_eq!(manifest.loader.loader_type, LoaderType::Fabric);
    assert_eq!(manifest.loader.version, "0.16.10");
    assert_eq!(manifest.required_java_version(), 21);
    assert_eq!(manifest.mods.len(), 3);

    // Verify Cobblemon mod entry
    let cobblemon = &manifest.mods[0];
    assert_eq!(cobblemon.name, "Cobblemon-fabric-1.8.1+1.21.1");
    assert_eq!(cobblemon.source, "modrinth");
    let (c_hash, c_algo) = cobblemon.primary_hash().expect("primary hash must exist");
    assert_eq!(c_algo, HashAlgorithm::Sha512);
    assert_eq!(
        c_hash,
        "e9c475c6a8e73bfb378445944812f6e31271cdf50d3227124b44b31c295bf7e9085246ea05d005efff9b7227e7b4e3e34ff1480b1a4b3b97211b5fa127d234ed"
    );

    // Verify Fabric API mod entry
    let fapi = &manifest.mods[1];
    assert_eq!(fapi.name, "fabric-api-0.116.17+1.21.1");
    assert_eq!(fapi.source, "modrinth");
    let (f_hash, f_algo) = fapi.primary_hash().expect("primary hash must exist");
    assert_eq!(f_algo, HashAlgorithm::Sha512);
    assert_eq!(
        f_hash,
        "98c478217da19181f0e4df3ea6c2da6bdbda271c7544a4aea048513a49b444f1ff5b71dc0df6303017c48e8ff69b7bb6af6179bdf0d0e09ab3c1ee08411857a8"
    );

    // Verify Architectury API mod entry
    let arch = &manifest.mods[2];
    assert_eq!(arch.name, "architectury-13.0.11-fabric");
    assert_eq!(arch.source, "modrinth");
    let (a_hash, a_algo) = arch.primary_hash().expect("primary hash must exist");
    assert_eq!(a_algo, HashAlgorithm::Sha512);
    assert_eq!(
        a_hash,
        "53314eb34890b11ad3c06c6f3ba2d01ae1ca756235ce4f0574e4b0544952a2ab3c628c81b521653d6487d62c5dc3055e06fed6367a3a499bafad909512f0c716"
    );
}

#[test]
fn test_hypixel_modern_manifest_validation() {
    let manifest_path = fixtures_dir().join("hypixel-26.2.json");
    let manifest = ServerManifest::from_file(&manifest_path)
        .expect("hypixel modern fixture should parse from file");

    manifest.validate().expect("hypixel modern fixture must be valid");
    assert_eq!(manifest.server_id, "hypixel-26-2");
    assert_eq!(manifest.name, "Hypixel Server in modern client");
    assert_eq!(manifest.server_address, "hypixel.net");
    assert_eq!(manifest.minecraft_version, "26.2");
    assert_eq!(manifest.loader.loader_type, LoaderType::Fabric);
    assert_eq!(manifest.loader.version, "0.20.1");
    assert_eq!(manifest.required_java_version(), 25);
    assert_eq!(manifest.mods.len(), 51);
    assert!(manifest.config.is_some());
}

#[test]
fn test_hypixel_legacy_manifest_validation() {
    let manifest_path = fixtures_dir().join("hypixel-1.8.9.json");
    let manifest = ServerManifest::from_file(&manifest_path)
        .expect("hypixel legacy fixture should parse from file");

    manifest.validate().expect("hypixel legacy fixture must be valid");
    assert_eq!(manifest.server_id, "hypixel-1-8-9");
    assert_eq!(manifest.name, "Hypixel Server in Legacy client");
    assert_eq!(manifest.server_address, "hypixel.net");
    assert_eq!(manifest.minecraft_version, "1.8.9");
    assert_eq!(manifest.loader.loader_type, LoaderType::Vanilla);
    assert_eq!(manifest.required_java_version(), 8);
    assert!(manifest.mods.is_empty());
}

#[test]
fn test_skyfactory4_manifest_validation() {
    let manifest_path = fixtures_dir().join("skyfactory4-1.12.2.json");
    let manifest = ServerManifest::from_file(&manifest_path)
        .expect("skyfactory4 fixture should parse from file");

    manifest.validate().expect("skyfactory4 fixture must be valid");
    assert_eq!(manifest.server_id, "skyfactory-4-1-12-2");
    assert_eq!(manifest.name, "SkyFactory 4");
    assert_eq!(manifest.server_address, "127.0.0.1:25565");
    assert_eq!(manifest.minecraft_version, "1.12.2");
    assert_eq!(manifest.loader.loader_type, LoaderType::Forge);
    assert_eq!(manifest.loader.version, "14.23.5.2860");
    assert_eq!(manifest.required_java_version(), 8);
    assert_eq!(manifest.mods.len(), 202);

    // Verify bedpatch mod entry
    let bedpatch = &manifest.mods[0];
    assert_eq!(bedpatch.name, "bedpatch-2.2-1.12.2");
    assert_eq!(bedpatch.source, "curseforge");
    let (b_hash, b_algo) = bedpatch.primary_hash().expect("primary hash must exist");
    assert_eq!(b_algo, HashAlgorithm::Sha1);
    assert_eq!(b_hash, "0c54d926ef72088e5f213a89e220d55a2284e44b");
}

#[test]
fn test_fixtures_server_address_quickplay_branching() {
    // 1. Cobblemon (1.21.1 >= 1.20) -> --quickPlayMultiplayer
    let config_cobblemon = LaunchConfig {
        java_binary: PathBuf::from("/mock/bin/java"),
        game_dir: PathBuf::from("/mock/servers/cobblemon-1-21-1"),
        assets_dir: PathBuf::from("/mock/assets"),
        asset_index: "1.21".to_string(),
        version: "1.21.1".to_string(),
        main_class: "net.fabricmc.loader.impl.launch.knot.KnotClient".to_string(),
        classpath: "fabric.jar:client.jar".to_string(),
        max_memory: "-Xmx2G".to_string(),
        username: "DevTester".to_string(),
        uuid: "00000000-0000-0000-0000-000000000000".to_string(),
        access_token: "mock-token".to_string(),
        user_type: "msa".to_string(),
        quick_play_multiplayer: Some("cobblemon.example.com".to_string()),
        extra_jvm_args: Vec::new(),
        extra_game_args: Vec::new(),
    };
    let cmd_c = build_command(&config_cobblemon);
    let cmd_str_c = format_command_line(&cmd_c);
    assert!(cmd_str_c.contains("--quickPlayMultiplayer cobblemon.example.com"));
    assert!(!cmd_str_c.contains("--server"));

    // 2. Hypixel Modern 26.2 (26.2 >= 1.20) -> --quickPlayMultiplayer
    let config_hypixel = LaunchConfig {
        java_binary: PathBuf::from("/mock/bin/java"),
        game_dir: PathBuf::from("/mock/servers/hypixel-26-2"),
        assets_dir: PathBuf::from("/mock/assets"),
        asset_index: "34".to_string(),
        version: "26.2".to_string(),
        main_class: "net.fabricmc.loader.impl.launch.knot.KnotClient".to_string(),
        classpath: "fabric.jar:client.jar".to_string(),
        max_memory: "-Xmx2G".to_string(),
        username: "DevTester".to_string(),
        uuid: "00000000-0000-0000-0000-000000000000".to_string(),
        access_token: "mock-token".to_string(),
        user_type: "msa".to_string(),
        quick_play_multiplayer: Some("hypixel.net".to_string()),
        extra_jvm_args: Vec::new(),
        extra_game_args: Vec::new(),
    };
    let cmd_p = build_command(&config_hypixel);
    let cmd_str_p = format_command_line(&cmd_p);
    assert!(cmd_str_p.contains("--quickPlayMultiplayer hypixel.net"));
    assert!(!cmd_str_p.contains("--server"));

    // 3. SkyFactory 4 (1.12.2 < 1.20) -> --server 127.0.0.1 --port 25565
    let config_sf4 = LaunchConfig {
        java_binary: PathBuf::from("/mock/bin/java"),
        game_dir: PathBuf::from("/mock/servers/skyfactory-4-1-12-2"),
        assets_dir: PathBuf::from("/mock/assets"),
        asset_index: "1.12".to_string(),
        version: "1.12.2".to_string(),
        main_class: "net.minecraft.launchwrapper.Launch".to_string(),
        classpath: "forge.jar:client.jar".to_string(),
        max_memory: "-Xmx2G".to_string(),
        username: "DevTester".to_string(),
        uuid: "00000000-0000-0000-0000-000000000000".to_string(),
        access_token: "mock-token".to_string(),
        user_type: "msa".to_string(),
        quick_play_multiplayer: Some("127.0.0.1:25565".to_string()),
        extra_jvm_args: vec!["-Dfml.ignoreInvalidMinecraftCertificates=true".to_string()],
        extra_game_args: vec![
            "--tweakClass".to_string(),
            "net.minecraftforge.fml.common.launcher.FMLTweaker".to_string(),
        ],
    };
    let cmd_sf4 = build_command(&config_sf4);
    let cmd_str_sf4 = format_command_line(&cmd_sf4);
    assert!(!cmd_str_sf4.contains("--quickPlayMultiplayer"));
    assert!(cmd_str_sf4.contains("--server 127.0.0.1"));
    assert!(cmd_str_sf4.contains("--port 25565"));
    assert!(cmd_str_sf4.contains("--tweakClass net.minecraftforge.fml.common.launcher.FMLTweaker"));
    assert!(cmd_str_sf4.contains("-Dfml.ignoreInvalidMinecraftCertificates=true"));
}

#[tokio::test]
async fn test_fixtures_dry_run_launch() {
    let paths = SzzPaths::new().expect("szz paths should be initialized");
    let auth = DevAuthSession::new("FixtureTester");
    let reporter = NoopProgressReporter;

    // 1. Dry run Hypixel Modern 26.2
    let hypixel_manifest = ServerManifest::from_file(&fixtures_dir().join("hypixel-26.2.json"))
        .expect("should load hypixel modern fixture");
    let hypixel_res = launch_server_manifest(&paths, &hypixel_manifest, &auth, &reporter, true)
        .await
        .expect("hypixel dry run should succeed");
    assert_eq!(hypixel_res.pid, None);
    assert!(hypixel_res.command_line.contains("net.fabricmc.loader.impl.launch.knot.KnotClient"));
    assert!(hypixel_res.command_line.contains("--quickPlayMultiplayer hypixel.net"));
    assert!(hypixel_res.command_line.contains("--version 26.2"));
    assert!(hypixel_res.command_line.contains("java-25"));

    // 2. Dry run Cobblemon 1.21.1
    let cobblemon_manifest = ServerManifest::from_file(&fixtures_dir().join("cobblemon-1.21.1.json"))
        .expect("should load cobblemon fixture");
    let cobblemon_res = launch_server_manifest(&paths, &cobblemon_manifest, &auth, &reporter, true)
        .await
        .expect("cobblemon dry run should succeed");
    assert_eq!(cobblemon_res.pid, None);
    assert!(cobblemon_res.command_line.contains("net.fabricmc.loader.impl.launch.knot.KnotClient"));
    assert!(cobblemon_res.command_line.contains("--quickPlayMultiplayer cobblemon.example.com"));
    assert!(cobblemon_res.command_line.contains("--version 1.21.1"));
    assert!(cobblemon_res.command_line.contains("java-21"));

    // 3. Dry run SkyFactory 4 1.12.2
    let sf4_manifest = ServerManifest::from_file(&fixtures_dir().join("skyfactory4-1.12.2.json"))
        .expect("should load skyfactory4 fixture");
    let sf4_res = launch_server_manifest(&paths, &sf4_manifest, &auth, &reporter, true)
        .await
        .expect("skyfactory4 dry run should succeed");
    assert_eq!(sf4_res.pid, None);
    assert!(sf4_res.command_line.contains("net.minecraft.launchwrapper.Launch"));
    assert!(sf4_res.command_line.contains("--server 127.0.0.1"));
    assert!(sf4_res.command_line.contains("--port 25565"));
    assert!(sf4_res.command_line.contains("java-8"));
}

#[tokio::test]
async fn test_fixtures_local_config_zip_sync() {
    let tmp = tempfile::tempdir().unwrap();
    let paths = SzzPaths::custom(tmp.path());
    paths.ensure_dirs().unwrap();

    // 1. Hypixel 26.2 local config zip extraction
    let hypixel_manifest = ServerManifest::from_file(&fixtures_dir().join("hypixel-26.2.json"))
        .expect("should load hypixel modern fixture");

    let handle = szz_core::config_sync::sync_server_config(&paths, &hypixel_manifest, false)
        .await
        .expect("local config sync should succeed for hypixel")
        .expect("handle should be returned");

    handle.commit().expect("commit should succeed");

    let instance_dir = paths.instance_dir(&hypixel_manifest.server_id);
    assert!(instance_dir.join("config/isxander-main-menu-credits.json").exists());
    assert!(instance_dir.join("config/modpack_defaults/options.txt").exists());
    assert!(instance_dir.join("resourcepacks/Mod Menu Helper.zip").exists());
    assert!(instance_dir.join(".szz_config_state.json").exists());

    // 2. SkyFactory 4 local config zip extraction
    let sf4_manifest = ServerManifest::from_file(&fixtures_dir().join("skyfactory4-1.12.2.json"))
        .expect("should load skyfactory4 fixture");

    let sf4_handle = szz_core::config_sync::sync_server_config(&paths, &sf4_manifest, false)
        .await
        .expect("local config sync should succeed for sf4")
        .expect("handle should be returned");

    sf4_handle.commit().expect("commit should succeed");

    let sf4_instance_dir = paths.instance_dir(&sf4_manifest.server_id);
    assert!(sf4_instance_dir.join(".szz_config_state.json").exists());
}
