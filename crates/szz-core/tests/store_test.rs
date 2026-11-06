use sha1::Sha1;
use sha2::{Digest, Sha512};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use szz_core::manifest::{HashAlgorithm, ModEntry};
use szz_core::paths::SzzPaths;
use szz_core::progress::{LaunchStep, ProgressReporter};
use szz_core::store::{
    are_same_file, clean_stale_temp_files, compute_file_hash, create_hardlink_or_copy,
    ensure_mod, sync_mods, sync_mods_with_reporter,
};
use tempfile::tempdir;

#[test]
fn test_compute_file_hash() {
    let tmp = tempdir().unwrap();
    let file_path = tmp.path().join("test.txt");
    let content = b"hello szz minecraft launcher";
    std::fs::write(&file_path, content).unwrap();

    let sha1 = compute_file_hash(&file_path, HashAlgorithm::Sha1).unwrap();
    let expected_sha1 = hex::encode(Sha1::digest(content));
    assert_eq!(sha1, expected_sha1);

    let sha512 = compute_file_hash(&file_path, HashAlgorithm::Sha512).unwrap();
    let expected_sha512 = hex::encode(Sha512::digest(content));
    assert_eq!(sha512, expected_sha512);
}

#[test]
fn test_hardlink_creation_and_inode_match() {
    let tmp = tempdir().unwrap();
    let source = tmp.path().join("store").join("mod.jar");
    let dest = tmp.path().join("instances").join("srv-1").join("mods").join("mod.jar");

    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    std::fs::write(&source, b"fake jar content").unwrap();

    create_hardlink_or_copy(&source, &dest).expect("hardlink should succeed");
    assert!(dest.exists());
    assert!(are_same_file(&source, &dest));
}

#[tokio::test]
async fn test_ensure_mod_uses_existing_cas_cache() {
    let tmp = tempdir().unwrap();
    let paths = SzzPaths::custom(tmp.path());
    paths.ensure_dirs().unwrap();

    let content = b"fabric-api-cached-binary";
    let hash = hex::encode(Sha1::digest(content));

    // Pre-populate CAS store
    let store_file = paths.store_dir().join(format!("{}.jar", hash));
    std::fs::write(&store_file, content).unwrap();

    let mod_entry = ModEntry {
        name: "Fabric API".to_string(),
        source: "modrinth".to_string(),
        file_url: "https://invalid-url-that-would-fail.example.com/mod.jar".to_string(),
        sha1: Some(hash),
        sha512: None,
    };

    let linked = ensure_mod(&paths, "test-server", &mod_entry)
        .await
        .expect("should reuse cached file without hitting invalid URL");

    assert!(linked.exists());
    assert!(are_same_file(&store_file, &linked));
}

#[test]
fn test_clean_stale_temp_files() {
    let tmp = tempdir().unwrap();
    let paths = SzzPaths::custom(tmp.path());
    std::fs::create_dir_all(paths.store_dir()).unwrap();

    let pid = std::process::id();
    let stale1 = paths.store_dir().join(format!("abc.tmp-{}-1", pid));
    let stale2 = paths.store_dir().join(format!("hash.tmp-{}-2", pid));
    let regular = paths.store_dir().join("legit-mod.jar");

    std::fs::write(&stale1, b"partial-1").unwrap();
    std::fs::write(&stale2, b"partial-2").unwrap();
    std::fs::write(&regular, b"valid-jar").unwrap();

    let removed = clean_stale_temp_files(&paths).unwrap();
    assert_eq!(removed, 2);

    assert!(!stale1.exists());
    assert!(!stale2.exists());
    assert!(regular.exists());
}

#[test]
fn test_clean_stale_temp_files_preserves_concurrent_other_process_fresh_temp() {
    let tmp = tempdir().unwrap();
    let paths = SzzPaths::custom(tmp.path());
    std::fs::create_dir_all(paths.store_dir()).unwrap();

    let pid = std::process::id();
    let other_pid = pid.wrapping_add(9999);
    let my_temp = paths.store_dir().join(format!("my.tmp-{}-1", pid));
    let other_fresh_temp = paths.store_dir().join(format!("other.tmp-{}-1", other_pid));

    std::fs::write(&my_temp, b"my-partial").unwrap();
    std::fs::write(&other_fresh_temp, b"other-active-partial").unwrap();

    let removed = clean_stale_temp_files(&paths).unwrap();
    assert_eq!(removed, 1);

    assert!(!my_temp.exists());
    // The other process's fresh temp file must NOT be deleted
    assert!(other_fresh_temp.exists());
}

#[tokio::test]
async fn test_mod_filename_sanitization_prevents_path_traversal() {
    let tmp = tempdir().unwrap();
    let paths = SzzPaths::custom(tmp.path());
    paths.ensure_dirs().unwrap();

    let content = b"mod-payload";
    let hash = hex::encode(Sha1::digest(content));

    // Pre-populate CAS store
    let store_file = paths.store_dir().join(format!("{}.jar", hash));
    std::fs::write(&store_file, content).unwrap();

    // Mod with malicious directory traversal in name and file_url
    let mods = vec![ModEntry {
        name: "../../malicious_escape".to_string(),
        source: "custom".to_string(),
        file_url: "https://example.com/../../pwned.jar".to_string(),
        sha1: Some(hash),
        sha512: None,
    }];

    let linked = sync_mods(&paths, "test-instance", &mods).await.unwrap();
    assert_eq!(linked.len(), 1);

    let linked_path = &linked[0];
    let mods_dir = paths.instance_mods_dir("test-instance");

    // Must be directly inside mods_dir, NOT escaped
    assert_eq!(linked_path.parent(), Some(mods_dir.as_path()));
    assert!(linked_path.exists());
}

#[tokio::test]
async fn test_sync_mods_deduplicates_same_hash() {
    let tmp = tempdir().unwrap();
    let paths = SzzPaths::custom(tmp.path());
    paths.ensure_dirs().unwrap();

    let content = b"common-shared-library";
    let hash = hex::encode(Sha1::digest(content));

    // Pre-populate CAS store
    let store_file = paths.store_dir().join(format!("{}.jar", hash));
    std::fs::write(&store_file, content).unwrap();

    // 2 mod entries with different names/filenames but the EXACT same primary hash
    let mods = vec![
        ModEntry {
            name: "Library A".to_string(),
            source: "modrinth".to_string(),
            file_url: "https://example.com/lib-a.jar".to_string(),
            sha1: Some(hash.clone()),
            sha512: None,
        },
        ModEntry {
            name: "Library A Alias".to_string(),
            source: "curseforge".to_string(),
            file_url: "https://example.com/lib-a-alias.jar".to_string(),
            sha1: Some(hash.clone()),
            sha512: None,
        },
    ];

    let linked = sync_mods(&paths, "test-instance", &mods).await.unwrap();
    assert_eq!(linked.len(), 2);

    let instance_mods = paths.instance_mods_dir("test-instance");
    let file1 = instance_mods.join("lib-a.jar");
    let file2 = instance_mods.join("lib-a-alias.jar");

    assert!(file1.exists());
    assert!(file2.exists());
    assert!(are_same_file(&store_file, &file1));
    assert!(are_same_file(&store_file, &file2));
}

struct TestReporter {
    download_reports: Arc<AtomicU32>,
    linking_reports: Arc<AtomicU32>,
}

impl ProgressReporter for TestReporter {
    fn report(&self, step: LaunchStep, msg: Option<&str>) {
        if step == LaunchStep::SyncingMods {
            if let Some(m) = msg {
                if m.starts_with("Downloading") {
                    self.download_reports.fetch_add(1, Ordering::SeqCst);
                } else if m.starts_with("Linked") {
                    self.linking_reports.fetch_add(1, Ordering::SeqCst);
                }
            }
        }
    }
}

async fn start_mock_mod_server(
    mods_content: HashMap<String, (u16, Vec<u8>)>,
) -> (String, tokio::task::JoinHandle<()>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    let handle = tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let mut buf = [0u8; 4096];
            if let Ok(n) = socket.read(&mut buf).await {
                if n == 0 {
                    continue;
                }
                let req = String::from_utf8_lossy(&buf[..n]);
                let path = req
                    .lines()
                    .next()
                    .and_then(|l| l.split_whitespace().nth(1))
                    .unwrap_or("");
                if let Some((status, body)) = mods_content.get(path) {
                    let resp = format!(
                        "HTTP/1.1 {} OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        status,
                        body.len()
                    );
                    let _ = socket.write_all(resp.as_bytes()).await;
                    let _ = socket.write_all(body).await;
                } else {
                    let resp =
                        "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                    let _ = socket.write_all(resp.as_bytes()).await;
                }
            }
        }
    });

    (base_url, handle)
}

#[tokio::test]
async fn test_sync_mods_parallel_download_with_pool_and_reporter() {
    let tmp = tempdir().unwrap();
    let paths = SzzPaths::custom(tmp.path());
    paths.ensure_dirs().unwrap();

    // Create 10 mock mods
    let mut server_files = HashMap::new();
    let mut mods = Vec::new();

    for i in 0..10 {
        let content = format!("mock mod content {}", i).into_bytes();
        let hash = hex::encode(Sha1::digest(&content));
        let path = format!("/mod-{}.jar", i);
        server_files.insert(path.clone(), (200u16, content));

        mods.push((path, hash, i));
    }

    let (base_url, server_handle) = start_mock_mod_server(server_files).await;

    let mod_entries: Vec<ModEntry> = mods
        .into_iter()
        .map(|(path, hash, i)| ModEntry {
            name: format!("Mod {}", i),
            source: "modrinth".to_string(),
            file_url: format!("{}{}", base_url, path),
            sha1: Some(hash),
            sha512: None,
        })
        .collect();

    let download_reports = Arc::new(AtomicU32::new(0));
    let linking_reports = Arc::new(AtomicU32::new(0));
    let reporter = TestReporter {
        download_reports: download_reports.clone(),
        linking_reports: linking_reports.clone(),
    };

    let linked = sync_mods_with_reporter(&paths, "test-instance", &mod_entries, &reporter)
        .await
        .unwrap();

    assert_eq!(linked.len(), 10);
    for p in &linked {
        assert!(p.exists());
    }

    // Verify progress was reported
    assert_eq!(download_reports.load(Ordering::SeqCst), 10);
    assert_eq!(linking_reports.load(Ordering::SeqCst), 1);

    server_handle.abort();
}

#[tokio::test]
async fn test_sync_mods_fail_fast_aborts_and_cleans_temp() {
    let tmp = tempdir().unwrap();
    let paths = SzzPaths::custom(tmp.path());
    paths.ensure_dirs().unwrap();

    let mut server_files = HashMap::new();
    // Mod 0 is valid
    let content0 = b"valid mod 0".to_vec();
    let hash0 = hex::encode(Sha1::digest(&content0));
    server_files.insert("/mod-0.jar".to_string(), (200u16, content0));

    // Mod 1 will return corrupted content (mismatched hash)
    let content1_corrupted = b"corrupted data".to_vec();
    server_files.insert("/mod-1.jar".to_string(), (200u16, content1_corrupted));

    let (base_url, server_handle) = start_mock_mod_server(server_files).await;

    let mod_entries = vec![
        ModEntry {
            name: "Mod 0".to_string(),
            source: "modrinth".to_string(),
            file_url: format!("{}/mod-0.jar", base_url),
            sha1: Some(hash0),
            sha512: None,
        },
        ModEntry {
            name: "Mod 1 Corrupted".to_string(),
            source: "modrinth".to_string(),
            file_url: format!("{}/mod-1.jar", base_url),
            sha1: Some("1111222233334444555566667777888899990000".to_string()),
            sha512: None,
        },
    ];

    let reporter = TestReporter {
        download_reports: Arc::new(AtomicU32::new(0)),
        linking_reports: Arc::new(AtomicU32::new(0)),
    };

    let res = sync_mods_with_reporter(&paths, "test-instance", &mod_entries, &reporter).await;
    assert!(res.is_err());

    // Verify fail-fast cleaned up any .tmp files
    let remaining_tmp = clean_stale_temp_files(&paths).unwrap();
    assert_eq!(remaining_tmp, 0);

    server_handle.abort();
}
