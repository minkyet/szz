use std::fs::File;
use szz_core::paths::SzzPaths;
use szz_core::runtime::{detect_os_and_arch, ensure_java_runtime, get_adoptium_download_url};
use tempfile::tempdir;

#[test]
fn test_os_arch_detection() {
    let (os, arch) = detect_os_and_arch().expect("os and arch should be detected");
    assert!(!os.is_empty());
    assert!(!arch.is_empty());
}

#[test]
fn test_adoptium_url() {
    let url = get_adoptium_download_url(17, "linux", "x64");
    assert_eq!(
        url,
        "https://api.adoptium.net/v3/binary/latest/17/ga/linux/x64/jre/hotspot/normal/eclipse"
    );
}

#[tokio::test]
async fn test_ensure_runtime_already_present_does_not_network() {
    let tmp = tempdir().unwrap();
    let paths = SzzPaths::custom(tmp.path());
    paths.ensure_dirs().unwrap();

    let binary = paths.java_binary_path(17);
    std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
    File::create(&binary).unwrap();

    let resolved = ensure_java_runtime(&paths, 17)
        .await
        .expect("should reuse existing runtime");

    assert_eq!(resolved, binary);
}
