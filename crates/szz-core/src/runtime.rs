use crate::error::{Result, SzzError};
use crate::paths::SzzPaths;
use flate2::read::GzDecoder;
use std::path::{Path, PathBuf};
use tar::Archive;

pub fn detect_os_and_arch() -> Result<(&'static str, &'static str)> {
    let os = if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "mac"
    } else {
        return Err(SzzError::Runtime(format!(
            "Unsupported operating system: {}",
            std::env::consts::OS
        )));
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        return Err(SzzError::Runtime(format!(
            "Unsupported CPU architecture: {}",
            std::env::consts::ARCH
        )));
    };

    Ok((os, arch))
}

pub fn get_adoptium_download_url(major_version: u32, os: &str, arch: &str) -> String {
    format!(
        "https://api.adoptium.net/v3/binary/latest/{}/ga/{}/{}/jre/hotspot/normal/eclipse",
        major_version, os, arch
    )
}

pub async fn ensure_java_runtime(paths: &SzzPaths, major_version: u32) -> Result<PathBuf> {
    let binary_path = paths.java_binary_path(major_version);
    if binary_path.exists() {
        return Ok(binary_path);
    }

    let (os, arch) = detect_os_and_arch()?;
    let download_url = get_adoptium_download_url(major_version, os, arch);

    println!(
        "[szz-core] Downloading isolated Java {} runtime from Adoptium ({}-{})...",
        major_version, os, arch
    );

    let client = reqwest::Client::builder()
        .user_agent("szz-launcher/0.1.0 (https://github.com/minkyet/szz)")
        .build()
        .map_err(|source| SzzError::Network {
            url: download_url.clone(),
            source,
        })?;

    let response = client
        .get(&download_url)
        .send()
        .await
        .map_err(|source| SzzError::Network {
            url: download_url.clone(),
            source,
        })?;

    if !response.status().is_success() {
        return Err(SzzError::Runtime(format!(
            "Adoptium API returned HTTP {} for {}",
            response.status(),
            download_url
        )));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|source| SzzError::Network {
            url: download_url.clone(),
            source,
        })?;

    let runtime_dir = paths.java_runtime_dir(major_version);
    let temp_extract_dir = paths.runtimes_dir().join(format!("java-{}.tmp", major_version));

    if temp_extract_dir.exists() {
        let _ = std::fs::remove_dir_all(&temp_extract_dir);
    }
    std::fs::create_dir_all(&temp_extract_dir).map_err(|source| SzzError::Io {
        path: temp_extract_dir.clone(),
        source,
    })?;

    if os == "windows" {
        extract_zip(&bytes, &temp_extract_dir)?;
    } else {
        extract_tar_gz(&bytes, &temp_extract_dir)?;
    }

    // Locate the inner java home directory (it typically has a 'bin' folder)
    let java_home = find_java_home(&temp_extract_dir)?
        .ok_or_else(|| SzzError::Runtime("Extracted archive does not contain a bin/ directory".to_string()))?;

    if runtime_dir.exists() {
        let _ = std::fs::remove_dir_all(&runtime_dir);
    }

    std::fs::rename(&java_home, &runtime_dir).map_err(|source| SzzError::Io {
        path: runtime_dir.clone(),
        source,
    })?;

    let _ = std::fs::remove_dir_all(&temp_extract_dir);

    let final_binary = paths.java_binary_path(major_version);
    if !final_binary.exists() {
        return Err(SzzError::Runtime(format!(
            "Java binary not found at expected path: {:?}",
            final_binary
        )));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&final_binary)
            .map_err(|source| SzzError::Io {
                path: final_binary.clone(),
                source,
            })?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&final_binary, perms).map_err(|source| SzzError::Io {
            path: final_binary.clone(),
            source,
        })?;
    }

    println!("[szz-core] Java {} runtime ready at {:?}", major_version, final_binary);
    Ok(final_binary)
}

fn extract_tar_gz(bytes: &[u8], target: &Path) -> Result<()> {
    let gz = GzDecoder::new(bytes);
    let mut archive = Archive::new(gz);
    archive.unpack(target).map_err(|source| SzzError::Io {
        path: target.to_path_buf(),
        source,
    })?;
    Ok(())
}

fn extract_zip(bytes: &[u8], target: &Path) -> Result<()> {
    let reader = std::io::Cursor::new(bytes);
    let mut zip =
        zip::ZipArchive::new(reader).map_err(|e| SzzError::Runtime(format!("Zip error: {}", e)))?;
    zip.extract(target)
        .map_err(|e| SzzError::Runtime(format!("Zip extraction error: {}", e)))?;
    Ok(())
}

fn find_java_home(dir: &Path) -> Result<Option<PathBuf>> {
    if dir.join("bin").exists() {
        return Ok(Some(dir.to_path_buf()));
    }

    let entries = std::fs::read_dir(dir).map_err(|source| SzzError::Io {
        path: dir.to_path_buf(),
        source,
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| SzzError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.is_dir() && path.join("bin").exists() {
            return Ok(Some(path));
        }
    }

    Ok(None)
}
