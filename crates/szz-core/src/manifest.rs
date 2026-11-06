use crate::error::{Result, SzzError};
use crate::loaders::natives::is_version_at_least;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoaderType {
    #[serde(alias = "Fabric")]
    Fabric,
    #[serde(alias = "Forge")]
    Forge,
    #[serde(alias = "NeoForge", alias = "neoforged", alias = "NeoForged")]
    NeoForge,
    #[serde(alias = "LiteLoader", alias = "liteloader")]
    LiteLoader,
    #[serde(alias = "Vanilla")]
    Vanilla,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoaderConfig {
    #[serde(rename = "type")]
    pub loader_type: LoaderType,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModEntry {
    pub name: String,
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(rename = "fileUrl")]
    pub file_url: String,
    pub sha1: Option<String>,
    pub sha512: Option<String>,
}

fn default_source() -> String {
    "custom".to_string()
}

impl ModEntry {
    /// Returns the primary hash to use for CAS storage and its algorithm.
    pub fn primary_hash(&self) -> Result<(&str, HashAlgorithm)> {
        if let Some(sha512) = &self.sha512 {
            if !HashAlgorithm::Sha512.is_valid_hex(sha512) {
                return Err(SzzError::Manifest(format!(
                    "Mod '{}' sha512 hash is invalid: expected 128-character hexadecimal string",
                    self.name
                )));
            }
            Ok((sha512.as_str(), HashAlgorithm::Sha512))
        } else if let Some(sha1) = &self.sha1 {
            if !HashAlgorithm::Sha1.is_valid_hex(sha1) {
                return Err(SzzError::Manifest(format!(
                    "Mod '{}' sha1 hash is invalid: expected 40-character hexadecimal string",
                    self.name
                )));
            }
            Ok((sha1.as_str(), HashAlgorithm::Sha1))
        } else {
            Err(SzzError::Manifest(format!(
                "Mod '{}' must provide at least sha1 or sha512 hash",
                self.name
            )))
        }
    }

    /// Determines the safe destination jar filename for this mod entry.
    pub fn filename(&self) -> String {
        let hash_lower = self
            .primary_hash()
            .map(|(h, _)| h.to_ascii_lowercase())
            .unwrap_or_default();

        let path_without_query = self
            .file_url
            .split('?')
            .next()
            .unwrap_or("")
            .split('#')
            .next()
            .unwrap_or("");

        let raw_name = path_without_query
            .split('/')
            .next_back()
            .and_then(|part| part.split('\\').next_back())
            .unwrap_or("");

        Path::new(raw_name)
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|n| {
                n.ends_with(".jar")
                    && !n.contains("..")
                    && !n.contains(':')
                    && !n.contains('*')
                    && !n.contains('?')
                    && !n.contains('"')
                    && !n.contains('<')
                    && !n.contains('>')
                    && !n.contains('|')
            })
            .map(ToString::to_string)
            .unwrap_or_else(|| {
                let safe_stem: String = self
                    .name
                    .chars()
                    .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
                    .collect();
                let trimmed = safe_stem.trim_matches('-');
                if trimmed.is_empty() {
                    format!("{}.jar", hash_lower)
                } else {
                    format!("{}.jar", trimmed)
                }
            })
    }

    /// Returns a unique identity key for this mod entry (primary lowercase hash, or filename fallback).
    pub fn identity_key(&self) -> String {
        self.primary_hash()
            .map(|(h, _)| h.to_ascii_lowercase())
            .unwrap_or_else(|_| self.filename())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    Sha1,
    Sha512,
}

impl HashAlgorithm {
    pub const fn expected_len(&self) -> usize {
        match self {
            HashAlgorithm::Sha1 => 40,
            HashAlgorithm::Sha512 => 128,
        }
    }

    pub fn is_valid_hex(&self, s: &str) -> bool {
        s.len() == self.expected_len() && s.chars().all(|c| c.is_ascii_hexdigit())
    }

    /// Parses a hash specification string (e.g. "sha1:...", "sha512:...", or raw 40/128 hex string).
    /// Returns `Some((clean_hex, algorithm))` if format and hex characters are valid.
    pub fn parse_spec(spec: &str) -> Option<(&str, Self)> {
        let (algo, clean) = if let Some(stripped) = spec.strip_prefix("sha1:") {
            (HashAlgorithm::Sha1, stripped)
        } else if let Some(stripped) = spec.strip_prefix("sha512:") {
            (HashAlgorithm::Sha512, stripped)
        } else if spec.len() == 40 {
            (HashAlgorithm::Sha1, spec)
        } else if spec.len() == 128 {
            (HashAlgorithm::Sha512, spec)
        } else {
            return None;
        };

        if algo.is_valid_hex(clean) {
            Some((clean, algo))
        } else {
            None
        }
    }

    /// Verifies in-memory bytes against an expected hex string.
    pub fn verify_bytes(&self, bytes: &[u8], expected_clean_hex: &str) -> bool {
        let actual = match self {
            HashAlgorithm::Sha1 => {
                use sha1::Digest;
                hex::encode(sha1::Sha1::digest(bytes))
            }
            HashAlgorithm::Sha512 => {
                use sha2::Digest;
                hex::encode(sha2::Sha512::digest(bytes))
            }
        };
        actual.eq_ignore_ascii_case(expected_clean_hex)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConfigSpec {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

impl<'de> Deserialize<'de> for ConfigSpec {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ConfigSpecHelper {
            Url(String),
            Object {
                url: String,
                #[serde(default)]
                hash: Option<String>,
                #[serde(default)]
                version: Option<String>,
            },
        }

        match ConfigSpecHelper::deserialize(deserializer)? {
            ConfigSpecHelper::Url(url) => Ok(ConfigSpec {
                url,
                hash: None,
                version: None,
            }),
            ConfigSpecHelper::Object { url, hash, version } => Ok(ConfigSpec { url, hash, version }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerManifest {
    #[serde(rename = "serverId")]
    pub server_id: String,
    pub name: String,
    #[serde(rename = "serverAddress")]
    pub server_address: String,
    #[serde(rename = "minecraftVersion")]
    pub minecraft_version: String,
    pub loader: LoaderConfig,
    #[serde(default, alias = "configUrl", alias = "config")]
    pub config: Option<ConfigSpec>,
    #[serde(rename = "manifestVersion")]
    pub manifest_version: u32,
    #[serde(default)]
    pub mods: Vec<ModEntry>,
}

impl std::str::FromStr for ServerManifest {
    type Err = SzzError;

    fn from_str(json: &str) -> std::result::Result<Self, Self::Err> {
        let manifest: ServerManifest = serde_json::from_str(json)?;
        manifest.validate()?;
        Ok(manifest)
    }
}

impl ServerManifest {
    pub fn from_json_str(json: &str) -> Result<Self> {
        use std::str::FromStr;
        Self::from_str(json)
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|source| SzzError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_json_str(&content)
    }

    pub fn config_url(&self) -> Option<&str> {
        self.config.as_ref().map(|c| c.url.as_str())
    }

    pub fn validate(&self) -> Result<()> {
        if self.server_id.trim().is_empty()
            || !self.server_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(SzzError::Manifest(
                "serverId must not be empty and must match ^[a-zA-Z0-9_-]+$".to_string(),
            ));
        }
        if self.server_address.trim().is_empty() {
            return Err(SzzError::Manifest(
                "serverAddress must not be empty".to_string(),
            ));
        }
        if self.minecraft_version.trim().is_empty() {
            return Err(SzzError::Manifest(
                "minecraftVersion must not be empty".to_string(),
            ));
        }
        if let Some(cfg) = &self.config {
            if cfg.url.trim().is_empty() {
                return Err(SzzError::Manifest("config url must not be empty".to_string()));
            }
            if let Some(h) = &cfg.hash {
                if HashAlgorithm::parse_spec(h).is_none() {
                    return Err(SzzError::Manifest(format!(
                        "config hash '{}' format unsupported: expected 40-char SHA-1 or 128-char SHA-512 hex",
                        h
                    )));
                }
            }
        }
        for (idx, m) in self.mods.iter().enumerate() {
            if m.file_url.trim().is_empty() {
                return Err(SzzError::Manifest(format!(
                    "mod[{}] ({}) fileUrl must not be empty",
                    idx, m.name
                )));
            }
            if let Some(sha1) = &m.sha1 {
                if !HashAlgorithm::Sha1.is_valid_hex(sha1) {
                    return Err(SzzError::Manifest(format!(
                        "mod[{}] ({}) sha1 must be a 40-character hexadecimal string",
                        idx, m.name
                    )));
                }
            }
            if let Some(sha512) = &m.sha512 {
                if !HashAlgorithm::Sha512.is_valid_hex(sha512) {
                    return Err(SzzError::Manifest(format!(
                        "mod[{}] ({}) sha512 must be a 128-character hexadecimal string",
                        idx, m.name
                    )));
                }
            }
            m.primary_hash()?;
        }
        Ok(())
    }

    /// Determines the required Java major version based on the Minecraft version.
    pub fn required_java_version(&self) -> u32 {
        let v = &self.minecraft_version;
        if is_version_at_least(v, "26.0") {
            25
        } else if is_version_at_least(v, "1.20.5") {
            21
        } else if is_version_at_least(v, "1.18") {
            17
        } else if is_version_at_least(v, "1.17") {
            16
        } else {
            8
        }
    }

    /// Returns true if all mods in the manifest originate from verified official platforms
    /// (Modrinth or CurseForge) and have cryptographic digests, or if the manifest has no mods.
    pub fn is_verified(&self) -> bool {
        self.mods.iter().all(|m| {
            let src = m.source.trim();
            (src.eq_ignore_ascii_case("modrinth") || src.eq_ignore_ascii_case("curseforge"))
                && m.primary_hash().is_ok()
        })
    }
}
