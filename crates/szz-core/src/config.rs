use crate::error::{Result, SzzError};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Searches for and loads environment variables from a `.env` file into the current process.
///
/// It checks in order:
/// 1. Current working directory and ancestor directories (up to 5 levels up)
/// 2. Directory containing the current executable
pub fn load_dotenv() {
    static LOADED: OnceLock<()> = OnceLock::new();
    LOADED.get_or_init(|| {
        if let Some(path) = find_dotenv_file() {
            let _ = load_env_file(&path);
        }
    });
}

/// Locates a `.env` file if one exists.
pub fn find_dotenv_file() -> Option<PathBuf> {
    if let Ok(mut dir) = std::env::current_dir() {
        for _ in 0..5 {
            let candidate = dir.join(".env");
            if candidate.is_file() {
                return Some(candidate);
            }
            if !dir.pop() {
                break;
            }
        }
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let candidate = exe_dir.join(".env");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

/// Parses `.env` formatted content into key-value pairs.
pub fn parse_dotenv_entries(content: &str) -> Vec<(String, String)> {
    let mut entries = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let clean_line = trimmed.strip_prefix("export ").unwrap_or(trimmed).trim();
        if let Some((key_part, val_part)) = clean_line.split_once('=') {
            let key = key_part.trim();
            let clean_val = val_part.split('#').next().unwrap_or("").trim();
            let val = clean_val
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .or_else(|| clean_val.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
                .unwrap_or(clean_val);

            if !key.is_empty() {
                entries.push((key.to_string(), val.to_string()));
            }
        }
    }

    entries
}

/// Loads a `.env` file from the given path and populates process environment variables
/// if they are not already set.
pub fn load_env_file(path: &Path) -> std::io::Result<()> {
    let content = std::fs::read_to_string(path)?;
    for (key, val) in parse_dotenv_entries(&content) {
        if std::env::var(&key).is_err() {
            std::env::set_var(key, val);
        }
    }
    Ok(())
}

fn get_env_or_dotenv(key: &str) -> Option<String> {
    if let Ok(runtime_val) = std::env::var(key) {
        let trimmed = runtime_val.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    load_dotenv();
    if let Ok(runtime_val) = std::env::var(key) {
        let trimmed = runtime_val.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    None
}

/// Returns the configured Microsoft OAuth Client ID.
///
/// Priority:
/// 1. Runtime environment variable: `SZZ_MSA_CLIENT_ID` (if developer sets it explicitly in shell)
/// 2. Compile-time embedded ID: `option_env!("SZZ_MSA_CLIENT_ID")` (baked in from `.env` by `build.rs` at build time)
/// 3. Runtime `.env` file (fallback for uncompiled dev setups)
///
/// Returns `Err(SzzError::Auth)` if no Client ID is provided.
pub fn msa_client_id() -> Result<String> {
    // 1. Runtime environment variable or runtime .env
    if let Some(val) = get_env_or_dotenv("SZZ_MSA_CLIENT_ID") {
        return Ok(val);
    }

    // 2. Compile-time embedded ID (automatically baked in from .env at build time by build.rs)
    if let Some(compile_val) = option_env!("SZZ_MSA_CLIENT_ID") {
        let trimmed = compile_val.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    Err(SzzError::Auth(
        "Microsoft OAuth Client ID가 설정되지 않았습니다. .env 파일 또는 환경 변수(SZZ_MSA_CLIENT_ID)를 설정해주세요.".to_string(),
    ))
}

pub const DEFAULT_CATALOG_URL: &str = "https://catalog.szz-mc.net/catalog.json";

/// Returns the remote server catalog URL, checking runtime environment variables and .env file,
/// falling back to `DEFAULT_CATALOG_URL`.
pub fn catalog_url() -> String {
    get_env_or_dotenv("SZZ_CATALOG_URL")
        .unwrap_or_else(|| DEFAULT_CATALOG_URL.to_string())
}
