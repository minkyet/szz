use szz_core::loaders::natives::{extract_natives, is_version_at_least, parse_server_address};
use szz_core::SzzError;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[test]
fn test_is_version_at_least() {
    assert!(is_version_at_least("1.20", "1.20"));
    assert!(is_version_at_least("1.20.1", "1.20"));
    assert!(is_version_at_least("1.21", "1.20"));
    assert!(!is_version_at_least("1.19.4", "1.20"));
    assert!(!is_version_at_least("1.12.2", "1.20"));

    assert!(is_version_at_least("1.18", "1.18"));
    assert!(is_version_at_least("1.18.2", "1.18"));
    assert!(!is_version_at_least("1.16.5", "1.18"));
    assert!(!is_version_at_least("1.12.2", "1.18"));

    assert!(is_version_at_least("1.13", "1.13"));
    assert!(is_version_at_least("1.16.5", "1.13"));
    assert!(!is_version_at_least("1.12.2", "1.13"));
    assert!(!is_version_at_least("1.7.10", "1.13"));
}

#[test]
fn test_parse_server_address() {
    // Standard domain with port
    let (h1, p1) = parse_server_address("play.example.com:25565");
    assert_eq!(h1, "play.example.com");
    assert_eq!(p1, 25565);

    // Standard domain without port (default 25565)
    let (h2, p2) = parse_server_address("mc.example.com");
    assert_eq!(h2, "mc.example.com");
    assert_eq!(p2, 25565);

    // IPv4 with custom port
    let (h3, p3) = parse_server_address("127.0.0.1:12345");
    assert_eq!(h3, "127.0.0.1");
    assert_eq!(p3, 12345);

    // IPv4 without port
    let (h4, p4) = parse_server_address("127.0.0.1");
    assert_eq!(h4, "127.0.0.1");
    assert_eq!(p4, 25565);

    // IPv6 with brackets and port
    let (h5, p5) = parse_server_address("[2001:db8::1]:25566");
    assert_eq!(h5, "2001:db8::1");
    assert_eq!(p5, 25566);

    // IPv6 with brackets without port
    let (h6, p6) = parse_server_address("[::1]");
    assert_eq!(h6, "::1");
    assert_eq!(p6, 25565);

    // Trimming whitespace
    let (h7, p7) = parse_server_address("  custom.server.net:8080  ");
    assert_eq!(h7, "custom.server.net");
    assert_eq!(p7, 8080);

    // Bare IPv6 without brackets (must not be truncated or split as port)
    let (h8, p8) = parse_server_address("2001:db8::1");
    assert_eq!(h8, "2001:db8::1");
    assert_eq!(p8, 25565);

    let (h9, p9) = parse_server_address("::1");
    assert_eq!(h9, "::1");
    assert_eq!(p9, 25565);
}

#[test]
fn test_extract_natives_safe() {
    let tmp = tempdir().expect("tempdir");
    let zip_path = tmp.path().join("lwjgl-natives.jar");
    let dest_dir = tmp.path().join("natives");

    // Create a mock native jar
    {
        let file = File::create(&zip_path).expect("create zip");
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        // 1. Valid native binary
        zip.start_file("liblwjgl.so", options).expect("start liblwjgl.so");
        zip.write_all(b"fake so binary").expect("write liblwjgl.so");

        // 2. Subdirectory native binary
        zip.start_file("x64/libopenal.so", options).expect("start x64/libopenal.so");
        zip.write_all(b"fake openal binary").expect("write x64/libopenal.so");

        // 3. META-INF entry (should be ignored)
        zip.start_file("META-INF/MANIFEST.MF", options).expect("start manifest");
        zip.write_all(b"Manifest-Version: 1.0\n").expect("write manifest");

        zip.finish().expect("finish zip");
    }

    let extracted = extract_natives(&dest_dir, &[zip_path]).expect("extract_natives");
    assert_eq!(extracted, dest_dir);

    // Verify extracted files
    assert!(dest_dir.join("liblwjgl.so").exists());
    assert_eq!(
        std::fs::read_to_string(dest_dir.join("liblwjgl.so")).unwrap(),
        "fake so binary"
    );
    assert!(dest_dir.join("x64/libopenal.so").exists());

    // Verify META-INF was NOT extracted
    assert!(!dest_dir.join("META-INF").exists());
}

#[test]
fn test_extract_natives_zip_slip_rejection() {
    let tmp = tempdir().expect("tempdir");
    let zip_path = tmp.path().join("malicious.jar");
    let dest_dir = tmp.path().join("natives");

    // Create a zip with path traversal entry
    {
        let file = File::create(&zip_path).expect("create zip");
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        // Malicious entry trying to escape dest_dir
        zip.start_file("../outside.txt", options).expect("start malicious file");
        zip.write_all(b"pwned").expect("write malicious file");

        zip.finish().expect("finish zip");
    }

    let result = extract_natives(&dest_dir, &[zip_path]);
    assert!(result.is_err());
    match result.unwrap_err() {
        SzzError::Security(msg) => {
            assert!(msg.contains("Zip Slip") || msg.contains("traversal"));
        }
        other => panic!("Expected SzzError::Security, got {:?}", other),
    }

    // Ensure the file was not written outside dest_dir
    assert!(!tmp.path().join("outside.txt").exists());
}
