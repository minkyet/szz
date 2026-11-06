use crate::error::{Result, SzzError};
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

/// Compares two dot-separated version strings (e.g. "1.20.1" vs "1.20", "1.12.2" vs "1.13").
/// Returns true if `version >= min_version`.
pub fn is_version_at_least(version: &str, min_version: &str) -> bool {
    let parse_parts = |s: &str| -> Vec<u32> {
        s.split('.')
            .map(|part| {
                // Strip any suffix like -rc1, -pre2
                let num_str: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
                num_str.parse::<u32>().unwrap_or(0)
            })
            .collect()
    };

    let v_parts = parse_parts(version);
    let min_parts = parse_parts(min_version);

    let max_len = v_parts.len().max(min_parts.len());
    for i in 0..max_len {
        let v = *v_parts.get(i).unwrap_or(&0);
        let m = *min_parts.get(i).unwrap_or(&0);
        if v > m {
            return true;
        } else if v < m {
            return false;
        }
    }
    true
}

const DEFAULT_MINECRAFT_PORT: u16 = 25565;

/// Parses a server address string into `(host, port)`.
/// Defaults port to 25565 if omitted.
/// Handles IPv4, IPv6 (with brackets), and domain names.
pub fn parse_server_address(addr: &str) -> (String, u16) {
    let trimmed = addr.trim();
    if trimmed.is_empty() {
        return ("127.0.0.1".to_string(), DEFAULT_MINECRAFT_PORT);
    }

    if trimmed.starts_with('[') {
        if let Some(close_bracket) = trimmed.find(']') {
            let host = &trimmed[1..close_bracket];
            let remainder = &trimmed[close_bracket + 1..];
            let port = remainder
                .strip_prefix(':')
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(DEFAULT_MINECRAFT_PORT);
            return (host.to_string(), port);
        }
    }

    // Bare IPv6 check (e.g. "2001:db8::1", "::1"): if multiple colons exist without brackets
    if trimmed.parse::<std::net::Ipv6Addr>().is_ok() || trimmed.matches(':').count() > 1 {
        return (trimmed.to_string(), DEFAULT_MINECRAFT_PORT);
    }

    if let Some((host_part, port_part)) = trimmed.rsplit_once(':') {
        // Ensure there's only one colon or this is a valid port number
        if let Ok(port) = port_part.parse::<u16>() {
            return (host_part.to_string(), port);
        }
    }

    (trimmed.to_string(), DEFAULT_MINECRAFT_PORT)
}

/// Extracts LWJGL native libraries from a list of jar/zip files into `dest_dir`.
/// Enforces Zip Slip path sanitization, symlink rejection, and excludes `META-INF/`.
pub fn extract_natives(dest_dir: &Path, native_jar_paths: &[PathBuf]) -> Result<PathBuf> {
    const MAX_NATIVE_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100 MB max per native file

    std::fs::create_dir_all(dest_dir).map_err(|source| SzzError::Io {
        path: dest_dir.to_path_buf(),
        source,
    })?;

    for jar_path in native_jar_paths {
        if !jar_path.exists() {
            continue;
        }

        let file = File::open(jar_path).map_err(|source| SzzError::Io {
            path: jar_path.clone(),
            source,
        })?;

        let mut archive = zip::ZipArchive::new(file).map_err(|e| {
            SzzError::Loader(format!("Failed to open native archive {:?}: {}", jar_path, e))
        })?;

        for i in 0..archive.len() {
            let mut zip_file = archive.by_index(i).map_err(|e| {
                SzzError::Loader(format!("Failed to read entry {} in {:?}: {}", i, jar_path, e))
            })?;

            // 1. Strict symlink rejection
            if zip_file.is_symlink()
                || zip_file
                    .unix_mode()
                    .is_some_and(|m| (m & 0o170000) == 0o120000)
            {
                return Err(SzzError::Security(format!(
                    "Symbolic link entry '{}' rejected in native archive {:?}",
                    zip_file.name(),
                    jar_path
                )));
            }

            // 2. Strict Zip Slip defense: enclosed_name() strips leading slashes and prevents '..'
            let enclosed = match zip_file.enclosed_name() {
                Some(p) => p.to_path_buf(),
                None => {
                    return Err(SzzError::Security(format!(
                        "Zip Slip directory traversal detected in entry '{}' of {:?}",
                        zip_file.name(),
                        jar_path
                    )));
                }
            };

            // 3. Exclude META-INF directory
            if enclosed.starts_with("META-INF") {
                continue;
            }

            let out_path = dest_dir.join(&enclosed);

            // 4. Double-check that out_path strictly starts with dest_dir
            if !out_path.starts_with(dest_dir) {
                return Err(SzzError::Security(format!(
                    "Path traversal out of bounds: {:?}",
                    out_path
                )));
            }

            if out_path.is_symlink() {
                return Err(SzzError::Security(format!(
                    "Destination path is an existing symlink: {:?}",
                    out_path
                )));
            }

            if zip_file.is_dir() {
                std::fs::create_dir_all(&out_path).map_err(|source| SzzError::Io {
                    path: out_path.clone(),
                    source,
                })?;
            } else {
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|source| SzzError::Io {
                        path: parent.to_path_buf(),
                        source,
                    })?;
                }

                let mut out_file = File::create(&out_path).map_err(|source| SzzError::Io {
                    path: out_path.clone(),
                    source,
                })?;

                let mut limited = std::io::Read::take(&mut zip_file, MAX_NATIVE_FILE_SIZE + 1);
                let bytes_copied = io::copy(&mut limited, &mut out_file).map_err(|source| SzzError::Io {
                    path: out_path.clone(),
                    source,
                })?;

                if bytes_copied > MAX_NATIVE_FILE_SIZE {
                    let _ = std::fs::remove_file(&out_path);
                    return Err(SzzError::Security(format!(
                        "Native file '{}' exceeded maximum allowed size of {} bytes",
                        enclosed.display(),
                        MAX_NATIVE_FILE_SIZE
                    )));
                }

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = std::fs::metadata(&out_path)
                        .map_err(|source| SzzError::Io {
                            path: out_path.clone(),
                            source,
                        })?
                        .permissions();
                    perms.set_mode(0o755);
                    let _ = std::fs::set_permissions(&out_path, perms);
                }
            }
        }
    }

    Ok(dest_dir.to_path_buf())
}
