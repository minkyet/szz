use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=SZZ_MSA_CLIENT_ID");

    // 1. If SZZ_MSA_CLIENT_ID is explicitly provided in build environment, use it
    if let Ok(val) = std::env::var("SZZ_MSA_CLIENT_ID") {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            println!("cargo:rustc-env=SZZ_MSA_CLIENT_ID={}", trimmed);
            return;
        }
    }

    // 2. Otherwise, look for .env in CARGO_MANIFEST_DIR or parent directories (workspace root)
    if let Some(env_file) = find_dotenv_file() {
        println!("cargo:rerun-if-changed={}", env_file.display());
        if let Ok(content) = std::fs::read_to_string(&env_file) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                let clean = trimmed.strip_prefix("export ").unwrap_or(trimmed).trim();
                if let Some((k, v)) = clean.split_once('=') {
                    if k.trim() == "SZZ_MSA_CLIENT_ID" {
                        let clean_val = v.split('#').next().unwrap_or("").trim();
                        let val = clean_val
                            .strip_prefix('"')
                            .and_then(|s| s.strip_suffix('"'))
                            .or_else(|| clean_val.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
                            .unwrap_or(clean_val);

                        if !val.is_empty() {
                            println!("cargo:rustc-env=SZZ_MSA_CLIENT_ID={}", val);
                        }
                        break;
                    }
                }
            }
        }
    }
}

fn find_dotenv_file() -> Option<PathBuf> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").ok()?;
    let mut dir = PathBuf::from(manifest_dir);
    for _ in 0..5 {
        let candidate = dir.join(".env");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    None
}
