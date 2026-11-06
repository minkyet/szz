use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};
#[cfg(unix)]
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Parses a raw URL string and extracts the target `serverId` if it matches `szz://launch/{id}`.
pub fn parse_deep_link_url(raw_url: &str) -> Option<String> {
    let trimmed = raw_url.trim();
    const PREFIX: &str = "szz://launch/";

    let server_id = trimmed.strip_prefix(PREFIX)?;
    let cleaned = server_id.trim_end_matches('/');

    // Validate using canonical szz-core instance grammar and length limit
    if cleaned.is_empty()
        || cleaned.len() > 64
        || szz_core::instance::validate_instance_id(cleaned).is_err()
    {
        return None;
    }

    Some(cleaned.to_string())
}

/// Resolves the socket path used for single-instance IPC coordination under ~/.szz/
pub fn get_ipc_socket_path() -> PathBuf {
    szz_core::paths::SzzPaths::new()
        .map(|p| p.root.join("szz.sock"))
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".szz").join("szz.sock")
        })
}

#[cfg(unix)]
pub async fn try_forward_to_existing_instance(url: &str) -> bool {
    let socket_path = get_ipc_socket_path();

    match UnixStream::connect(&socket_path).await {
        Ok(mut stream) => {
            let msg = format!("{}\n", url);
            if stream.write_all(msg.as_bytes()).await.is_ok() {
                eprintln!("[szz-desktop] Forwarded deep-link to existing running instance: {}", url);
                return true;
            }
            false
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => {
            // Stale socket from previous crash; remove it
            let _ = std::fs::remove_file(&socket_path);
            false
        }
    }
}

#[cfg(not(unix))]
pub async fn try_forward_to_existing_instance(_url: &str) -> bool {
    false
}

#[cfg(unix)]
pub fn spawn_single_instance_listener(app_handle: AppHandle) {
    let socket_path = get_ipc_socket_path();

    tauri::async_runtime::spawn(async move {
        // Ensure parent directory ~/.szz exists
        if let Some(parent) = socket_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Only remove stale socket file if no active process responds
        if socket_path.exists() {
            let is_active = std::os::unix::net::UnixStream::connect(&socket_path).is_ok();
            if !is_active {
                let _ = std::fs::remove_file(&socket_path);
            }
        }

        let listener = match UnixListener::bind(&socket_path) {
            Ok(l) => {
                // Restrict socket permissions to current user
                let _ = std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600));
                l
            }
            Err(e) => {
                eprintln!("[szz-desktop] Warning: Could not bind single-instance IPC socket: {}", e);
                return;
            }
        };

        loop {
            match listener.accept().await {
                Ok((mut stream, _)) => {
                    let handle = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        let mut buf = [0u8; 1024];
                        if let Ok(Ok(n)) = tokio::time::timeout(
                            std::time::Duration::from_secs(2),
                            stream.read(&mut buf),
                        )
                        .await
                        {
                            if n > 0 {
                                let text = String::from_utf8_lossy(&buf[..n]);
                                for line in text.lines() {
                                    if let Some(server_id) = parse_deep_link_url(line) {
                                        eprintln!("[szz-desktop] Received deep-link for server: {}", server_id);

                                        // Focus and unminimize existing window
                                        if let Some(window) = handle.get_webview_window("main") {
                                            let _ = window.unminimize();
                                            let _ = window.show();
                                            let _ = window.set_focus();
                                        }

                                        // Emit event to frontend
                                        let _ = handle.emit("szz://deep-link", server_id);
                                    }
                                }
                            }
                        }
                    });
                }
                Err(e) => {
                    eprintln!("[szz-desktop] Single instance listener accept error: {}", e);
                    break;
                }
            }
        }

        let _ = std::fs::remove_file(&socket_path);
    });
}

#[cfg(not(unix))]
pub fn spawn_single_instance_listener(_app_handle: AppHandle) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_deep_link_url() {
        assert_eq!(
            parse_deep_link_url("szz://launch/cobblemon-1-21-1"),
            Some("cobblemon-1-21-1".to_string())
        );
        assert_eq!(
            parse_deep_link_url("szz://launch/vanilla-paper-26-3/"),
            Some("vanilla-paper-26-3".to_string())
        );
        assert_eq!(
            parse_deep_link_url("  szz://launch/skyfactory_4  "),
            Some("skyfactory_4".to_string())
        );
    }

    #[test]
    fn test_parse_invalid_schemes() {
        assert_eq!(parse_deep_link_url("http://szz.nz/launch/test"), None);
        assert_eq!(parse_deep_link_url("szz://other/test"), None);
        assert_eq!(parse_deep_link_url("szz://launch/"), None);
        assert_eq!(parse_deep_link_url(""), None);
    }

    #[test]
    fn test_parse_malicious_characters_rejected() {
        assert_eq!(parse_deep_link_url("szz://launch/../../etc/passwd"), None);
        assert_eq!(parse_deep_link_url("szz://launch/server;rm -rf /"), None);
        assert_eq!(parse_deep_link_url("szz://launch/server$VAR"), None);
        assert_eq!(parse_deep_link_url("szz://launch/server.with.dots"), None);
        assert_eq!(parse_deep_link_url(&format!("szz://launch/{}", "a".repeat(65))), None);
    }
}
