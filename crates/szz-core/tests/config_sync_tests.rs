use std::io::Write;
use std::path::{Path, PathBuf};
use szz_core::config_sync::{
    apply_config_archive, is_dangerous_executable, is_protected_user_path,
    is_sensitive_system_path, is_user_customizable_config,
};
use szz_core::manifest::ConfigSpec;
use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

fn create_zip_archive<F>(builder: F) -> Vec<u8>
where
    F: FnOnce(&mut ZipWriter<std::io::Cursor<Vec<u8>>>),
{
    let cursor = std::io::Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);
    builder(&mut zip);
    zip.finish().unwrap().into_inner()
}

#[test]
fn test_security_guards() {
    // Executables
    assert!(is_dangerous_executable(Path::new("payload.exe")));
    assert!(is_dangerous_executable(Path::new("script.bat")));
    assert!(is_dangerous_executable(Path::new("run.CMD")));
    assert!(is_dangerous_executable(Path::new("setup.sh")));
    assert!(is_dangerous_executable(Path::new("attack.ps1")));
    assert!(!is_dangerous_executable(Path::new("config.json")));
    assert!(!is_dangerous_executable(Path::new("rules.properties")));
    assert!(!is_dangerous_executable(Path::new("rules.txt")));

    // Sensitive system paths
    assert!(is_sensitive_system_path(Path::new(".git/config")));
    assert!(is_sensitive_system_path(Path::new(".szz_config_state.json")));
    assert!(is_sensitive_system_path(Path::new("servers.dat")));
    assert!(is_sensitive_system_path(Path::new("launcher_profiles.json")));
    assert!(is_sensitive_system_path(Path::new(".bashrc")));
    assert!(!is_sensitive_system_path(Path::new("config/jei.json")));
    assert!(!is_sensitive_system_path(Path::new("modernfix/test.rules")));

    // Protected user paths
    assert!(is_protected_user_path(Path::new("saves/world1/level.dat")));
    assert!(is_protected_user_path(Path::new("screenshots/2026-10-01.png")));
    assert!(is_protected_user_path(Path::new("logs/latest.log")));
    assert!(is_protected_user_path(Path::new("xaerominimap/waypoints.txt")));
    assert!(!is_protected_user_path(Path::new("config/mod.json")));
    assert!(!is_protected_user_path(Path::new("shaderpacks/bsl.zip")));
}

#[test]
fn test_zip_slip_rejected() {
    let tmp = tempdir().unwrap();
    let instance_dir = tmp.path().join("instance");
    let config_spec = ConfigSpec {
        url: "https://example.com/config.zip".to_string(),
        hash: None,
        version: Some("1.0".to_string()),
    };

    let zip_bytes = create_zip_archive(|zip| {
        let options = SimpleFileOptions::default();
        zip.start_file("../../evil.txt", options).unwrap();
        zip.write_all(b"evil").unwrap();
    });

    let res = apply_config_archive(&zip_bytes, &instance_dir, &config_spec, false);
    assert!(res.is_err());
    let err_str = res.err().unwrap().to_string();
    assert!(err_str.contains("Zip-slip") || err_str.contains("Malicious") || err_str.contains("traversal"));
    assert!(!tmp.path().join("evil.txt").exists());
}

#[test]
fn test_symlink_rejected() {
    let tmp = tempdir().unwrap();
    let instance_dir = tmp.path().join("instance");
    let config_spec = ConfigSpec {
        url: "https://example.com/config.zip".to_string(),
        hash: None,
        version: Some("1.0".to_string()),
    };

    let zip_bytes = create_zip_archive(|zip| {
        let options = SimpleFileOptions::default();
        zip.add_symlink("link_to_root", "/etc/passwd", options).unwrap();
    });

    let res = apply_config_archive(&zip_bytes, &instance_dir, &config_spec, false);
    assert!(res.is_err());
    let err_str = res.err().unwrap().to_string();
    assert!(err_str.contains("Symbolic or hard links"));
}

#[test]
fn test_cas_mods_and_executables_dropped() {
    let tmp = tempdir().unwrap();
    let instance_dir = tmp.path().join("instance");
    let config_spec = ConfigSpec {
        url: "https://example.com/config.zip".to_string(),
        hash: None,
        version: Some("1.0".to_string()),
    };

    let zip_bytes = create_zip_archive(|zip| {
        let options = SimpleFileOptions::default();
        // Allowed
        zip.start_file("overrides/config/jei.json", options).unwrap();
        zip.write_all(b"{\"jei\": true}").unwrap();

        zip.start_file("overrides/modernfix/modernfix.properties", options).unwrap();
        zip.write_all(b"mixin=true").unwrap();

        // Dropped (mods/ CAS bypass attempt)
        zip.start_file("overrides/mods/bypass.jar", options).unwrap();
        zip.write_all(b"fake-jar").unwrap();

        // Dropped (libraries/ CAS bypass attempt)
        zip.start_file("overrides/libraries/bypass.jar", options).unwrap();
        zip.write_all(b"fake-lib").unwrap();

        // Dropped (executable attack)
        zip.start_file("overrides/run.bat", options).unwrap();
        zip.write_all(b"@echo evil").unwrap();
    });

    let handle = apply_config_archive(&zip_bytes, &instance_dir, &config_spec, false).unwrap();
    handle.commit().unwrap();

    assert!(instance_dir.join("config/jei.json").exists());
    assert!(instance_dir.join("modernfix/modernfix.properties").exists());
    assert!(!instance_dir.join("mods/bypass.jar").exists());
    assert!(!instance_dir.join("libraries/bypass.jar").exists());
    assert!(!instance_dir.join("run.bat").exists());
}

#[test]
fn test_options_txt_smart_lifecycle() {
    let tmp = tempdir().unwrap();
    let instance_dir = tmp.path().join("instance");
    let config_spec_v1 = ConfigSpec {
        url: "https://example.com/v1.zip".to_string(),
        hash: None,
        version: Some("1.0".to_string()),
    };

    // 1. Initial install with options.txt
    let zip_v1 = create_zip_archive(|zip| {
        let options = SimpleFileOptions::default();
        zip.start_file("overrides/options.txt", options).unwrap();
        zip.write_all(b"guiScale:2\nsoundVolume:0.8\n").unwrap();
        zip.start_file("overrides/config/v1.json", options).unwrap();
        zip.write_all(b"{}").unwrap();
    });

    let handle_v1 = apply_config_archive(&zip_v1, &instance_dir, &config_spec_v1, false).unwrap();
    handle_v1.commit().unwrap();

    let options_file = instance_dir.join("options.txt");
    assert!(options_file.exists());
    assert_eq!(
        std::fs::read_to_string(&options_file).unwrap(),
        "guiScale:2\nsoundVolume:0.8\n"
    );

    // 2. User customizes options.txt
    std::fs::write(&options_file, "guiScale:3\nsoundVolume:0.2\nuserKey:42\n").unwrap();

    // 3. Update to v2 with different default options.txt in zip
    let config_spec_v2 = ConfigSpec {
        url: "https://example.com/v2.zip".to_string(),
        hash: None,
        version: Some("2.0".to_string()),
    };
    let zip_v2 = create_zip_archive(|zip| {
        let options = SimpleFileOptions::default();
        zip.start_file("overrides/options.txt", options).unwrap();
        zip.write_all(b"guiScale:1\nsoundVolume:1.0\n").unwrap();
        zip.start_file("overrides/config/v2.json", options).unwrap();
        zip.write_all(b"{}").unwrap();
    });

    let handle_v2 = apply_config_archive(&zip_v2, &instance_dir, &config_spec_v2, false).unwrap();
    handle_v2.commit().unwrap();

    // Verify user options are 100% preserved
    assert_eq!(
        std::fs::read_to_string(&options_file).unwrap(),
        "guiScale:3\nsoundVolume:0.2\nuserKey:42\n"
    );
    assert!(instance_dir.join("config/v2.json").exists());
}

#[test]
fn test_rolling_backup_and_pruning() {
    let tmp = tempdir().unwrap();
    let instance_dir = tmp.path().join("instance");

    // Perform 5 successive updates with different configs
    for i in 1..=5 {
        let config_spec = ConfigSpec {
            url: format!("https://example.com/v{}.zip", i),
            hash: None,
            version: Some(format!("{}.0", i)),
        };
        let zip_bytes = create_zip_archive(|zip| {
            let options = SimpleFileOptions::default();
            zip.start_file(format!("overrides/config/v{}.json", i), options).unwrap();
            zip.write_all(format!("v{}", i).as_bytes()).unwrap();
        });

        // Small sleep to ensure distinct timestamps
        std::thread::sleep(std::time::Duration::from_millis(50));
        let handle = apply_config_archive(&zip_bytes, &instance_dir, &config_spec, false).unwrap();
        handle.commit().unwrap();
    }

    let backups_dir = instance_dir.join(".szz_backups");
    assert!(backups_dir.exists());

    let mut backups: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(&backups_dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name().to_string_lossy().starts_with("overrides_") {
            backups.push(entry.path());
        }
    }

    // Must be at most 3 backups
    assert_eq!(backups.len(), 3);

    // Also verify prune_stale_managed_files: previous versions (v1, v2, v3, v4) were pruned
    assert!(!instance_dir.join("config/v1.json").exists());
    assert!(!instance_dir.join("config/v2.json").exists());
    assert!(!instance_dir.join("config/v3.json").exists());
    assert!(!instance_dir.join("config/v4.json").exists());
    assert!(instance_dir.join("config/v5.json").exists());
}

#[test]
fn test_dry_run_does_not_mutate_instance() {
    let tmp = tempdir().unwrap();
    let instance_dir = tmp.path().join("instance");
    let config_spec = ConfigSpec {
        url: "https://example.com/v1.zip".to_string(),
        hash: None,
        version: Some("1.0".to_string()),
    };

    let zip_bytes = create_zip_archive(|zip| {
        let options = SimpleFileOptions::default();
        zip.start_file("overrides/config/test.json", options).unwrap();
        zip.write_all(b"{\"dry\": true}").unwrap();
    });

    let handle = apply_config_archive(&zip_bytes, &instance_dir, &config_spec, true).unwrap();
    assert_eq!(handle.state.managed_files, vec!["config/test.json"]);
    assert!(!instance_dir.join("config/test.json").exists());
    assert!(!instance_dir.join(".szz_config_state.json").exists());
}

#[test]
fn test_verify_bytes_hash() {
    use szz_core::config_sync::verify_bytes_hash;

    let data = b"hello world";
    // sha1: 2aae6c35c94fcfb415dbe95f408b9ce91ee846ed
    // sha512: 309ecc489c12d6eb4cc40f50c902f2b4d0ed77ee511a7c7a9bcd3ca86d4cd86f989dd35bc5ff499670da34255b45b0cfd830e81f605dcf7dc5542e93ae9cd76f

    assert!(verify_bytes_hash(data, "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed").is_ok());
    assert!(verify_bytes_hash(data, "sha1:2aae6c35c94fcfb415dbe95f408b9ce91ee846ed").is_ok());
    assert!(verify_bytes_hash(data, "309ecc489c12d6eb4cc40f50c902f2b4d0ed77ee511a7c7a9bcd3ca86d4cd86f989dd35bc5ff499670da34255b45b0cfd830e81f605dcf7dc5542e93ae9cd76f").is_ok());
    assert!(verify_bytes_hash(data, "sha512:309ecc489c12d6eb4cc40f50c902f2b4d0ed77ee511a7c7a9bcd3ca86d4cd86f989dd35bc5ff499670da34255b45b0cfd830e81f605dcf7dc5542e93ae9cd76f").is_ok());

    // Mismatches & invalid formats
    assert!(verify_bytes_hash(data, "0000000000000000000000000000000000000000").is_err());
    assert!(verify_bytes_hash(data, "invalid-length").is_err());
    assert!(verify_bytes_hash(data, "2aae6c35c94fcfb415dbe95f408b9ce91ee846zz").is_err());
}

#[test]
fn test_is_user_customizable_config_classification() {
    // Root options (including relative and leading slashes)
    assert!(is_user_customizable_config(Path::new("options.txt")));
    assert!(is_user_customizable_config(Path::new("OPTIONS.TXT")));
    assert!(is_user_customizable_config(Path::new("./options.txt")));
    assert!(is_user_customizable_config(Path::new("/options.txt")));
    assert!(is_user_customizable_config(Path::new(".\\optionsshaders.txt")));
    assert!(is_user_customizable_config(Path::new("optionsshaders.txt")));
    assert!(is_user_customizable_config(Path::new("OptionsShaders.txt")));

    // Core renderer and shader configs (Iris / Sodium)
    assert!(is_user_customizable_config(Path::new("config/iris.properties")));
    assert!(is_user_customizable_config(Path::new("./config/iris.properties")));
    assert!(is_user_customizable_config(Path::new("/config/sodium-options.json")));
    assert!(is_user_customizable_config(Path::new("config/sodium-options.json")));

    // Shaderpacks configuration text/properties
    assert!(is_user_customizable_config(Path::new("shaderpacks/bsl.txt")));
    assert!(is_user_customizable_config(Path::new("./shaderpacks/bsl.txt")));
    assert!(is_user_customizable_config(Path::new("shaderpacks/bsl.properties")));
    assert!(is_user_customizable_config(Path::new("shaderpacks/custom/options.txt")));

    // NOT user customizable configs (e.g. archive zip packs or general mod configs)
    assert!(!is_user_customizable_config(Path::new("shaderpacks/bsl.zip")));
    assert!(!is_user_customizable_config(Path::new("resourcepacks/server.zip")));
    assert!(!is_user_customizable_config(Path::new("config/jei.json")));
    assert!(!is_user_customizable_config(Path::new("config/other.properties")));
    assert!(!is_user_customizable_config(Path::new("mods/test.jar")));
}

#[test]
fn test_user_customizable_configs_preserved_while_server_packs_updated() {
    let tmp = tempdir().unwrap();
    let instance_dir = tmp.path().join("instance");

    // 1. First installation (v1)
    let config_spec_v1 = ConfigSpec {
        url: "https://example.com/v1.zip".to_string(),
        hash: None,
        version: Some("1.0".to_string()),
    };

    let zip_v1 = create_zip_archive(|zip| {
        let options = SimpleFileOptions::default();
        zip.start_file("overrides/optionsshaders.txt", options).unwrap();
        zip.write_all(b"shaderPack=BSL\n").unwrap();

        zip.start_file("overrides/config/iris.properties", options).unwrap();
        zip.write_all(b"enableShaders=true\nshaderPack=BSL\n").unwrap();

        zip.start_file("overrides/config/sodium-options.json", options).unwrap();
        zip.write_all(b"{\"maxFps\": 120}").unwrap();

        zip.start_file("overrides/shaderpacks/bsl.txt", options).unwrap();
        zip.write_all(b"profile=HIGH\n").unwrap();

        zip.start_file("overrides/resourcepacks/server.zip", options).unwrap();
        zip.write_all(b"server-resource-pack-v1").unwrap();

        zip.start_file("overrides/config/server-rules.json", options).unwrap();
        zip.write_all(b"{\"rule\": 1}").unwrap();
    });

    let handle_v1 = apply_config_archive(&zip_v1, &instance_dir, &config_spec_v1, false).unwrap();
    handle_v1.commit().unwrap();

    // Verify initial values
    assert_eq!(
        std::fs::read_to_string(instance_dir.join("optionsshaders.txt")).unwrap(),
        "shaderPack=BSL\n"
    );
    assert_eq!(
        std::fs::read_to_string(instance_dir.join("config/iris.properties")).unwrap(),
        "enableShaders=true\nshaderPack=BSL\n"
    );
    assert_eq!(
        std::fs::read_to_string(instance_dir.join("config/sodium-options.json")).unwrap(),
        "{\"maxFps\": 120}"
    );
    assert_eq!(
        std::fs::read_to_string(instance_dir.join("shaderpacks/bsl.txt")).unwrap(),
        "profile=HIGH\n"
    );
    assert_eq!(
        std::fs::read_to_string(instance_dir.join("resourcepacks/server.zip")).unwrap(),
        "server-resource-pack-v1"
    );
    assert_eq!(
        std::fs::read_to_string(instance_dir.join("config/server-rules.json")).unwrap(),
        "{\"rule\": 1}"
    );

    // 2. User customizes their settings
    std::fs::write(instance_dir.join("optionsshaders.txt"), "shaderPack=Complementary\n").unwrap();
    std::fs::write(
        instance_dir.join("config/iris.properties"),
        "enableShaders=true\nshaderPack=Complementary\n",
    )
    .unwrap();
    std::fs::write(
        instance_dir.join("config/sodium-options.json"),
        "{\"maxFps\": 240}",
    )
    .unwrap();
    std::fs::write(instance_dir.join("shaderpacks/bsl.txt"), "profile=ULTRA\n").unwrap();

    // 3. Second installation (v2 update from server)
    let config_spec_v2 = ConfigSpec {
        url: "https://example.com/v2.zip".to_string(),
        hash: None,
        version: Some("2.0".to_string()),
    };

    let zip_v2 = create_zip_archive(|zip| {
        let options = SimpleFileOptions::default();
        // Server tries to reset optionsshaders, iris, sodium, bsl.txt to defaults
        zip.start_file("overrides/optionsshaders.txt", options).unwrap();
        zip.write_all(b"shaderPack=ServerDefault\n").unwrap();

        zip.start_file("overrides/config/iris.properties", options).unwrap();
        zip.write_all(b"enableShaders=false\nshaderPack=ServerDefault\n").unwrap();

        zip.start_file("overrides/config/sodium-options.json", options).unwrap();
        zip.write_all(b"{\"maxFps\": 60}").unwrap();

        zip.start_file("overrides/shaderpacks/bsl.txt", options).unwrap();
        zip.write_all(b"profile=LOW\n").unwrap();

        // Server updates official resourcepack and server rules
        zip.start_file("overrides/resourcepacks/server.zip", options).unwrap();
        zip.write_all(b"server-resource-pack-v2").unwrap();

        zip.start_file("overrides/config/server-rules.json", options).unwrap();
        zip.write_all(b"{\"rule\": 2}").unwrap();
    });

    let handle_v2 = apply_config_archive(&zip_v2, &instance_dir, &config_spec_v2, false).unwrap();
    handle_v2.commit().unwrap();

    // Verify user customized configs are 100% PRESERVED
    assert_eq!(
        std::fs::read_to_string(instance_dir.join("optionsshaders.txt")).unwrap(),
        "shaderPack=Complementary\n"
    );
    assert_eq!(
        std::fs::read_to_string(instance_dir.join("config/iris.properties")).unwrap(),
        "enableShaders=true\nshaderPack=Complementary\n"
    );
    assert_eq!(
        std::fs::read_to_string(instance_dir.join("config/sodium-options.json")).unwrap(),
        "{\"maxFps\": 240}"
    );
    assert_eq!(
        std::fs::read_to_string(instance_dir.join("shaderpacks/bsl.txt")).unwrap(),
        "profile=ULTRA\n"
    );

    // Verify official server resourcepack and general mod config ARE UPDATED
    assert_eq!(
        std::fs::read_to_string(instance_dir.join("resourcepacks/server.zip")).unwrap(),
        "server-resource-pack-v2"
    );
    assert_eq!(
        std::fs::read_to_string(instance_dir.join("config/server-rules.json")).unwrap(),
        "{\"rule\": 2}"
    );

    // 4. Third installation: server removes server-rules.json and optionsshaders.txt from zip
    let config_spec_v3 = ConfigSpec {
        url: "https://example.com/v3.zip".to_string(),
        hash: None,
        version: Some("3.0".to_string()),
    };
    let zip_v3 = create_zip_archive(|zip| {
        let options = SimpleFileOptions::default();
        zip.start_file("overrides/resourcepacks/server.zip", options).unwrap();
        zip.write_all(b"server-resource-pack-v3").unwrap();
    });

    let handle_v3 = apply_config_archive(&zip_v3, &instance_dir, &config_spec_v3, false).unwrap();
    handle_v3.commit().unwrap();

    // server-rules.json should be pruned
    assert!(!instance_dir.join("config/server-rules.json").exists());
    // But user customizable configs must NOT be pruned
    assert!(instance_dir.join("optionsshaders.txt").exists());
    assert!(instance_dir.join("config/iris.properties").exists());
    assert!(instance_dir.join("config/sodium-options.json").exists());
    assert!(instance_dir.join("shaderpacks/bsl.txt").exists());
}
