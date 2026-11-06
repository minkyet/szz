use crate::error::{Result, SzzError};
use crate::loaders::natives::extract_natives;
use crate::loaders::ResolvedLaunchMeta;
use crate::paths::SzzPaths;
use serde::{Deserialize, Serialize};
use sha1::Digest;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

static TEMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub(crate) fn current_platform_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}

pub(crate) fn evaluate_rules(rules: &[MojangRule]) -> bool {
    if rules.is_empty() {
        return true;
    }
    let current_os = current_platform_name();
    let mut allowed = false;
    for rule in rules {
        let os_match = match &rule.os {
            Some(rule_os) => rule_os.name.as_deref() == Some(current_os),
            None => true,
        };
        if os_match {
            if rule.action == "allow" {
                allowed = true;
            } else if rule.action == "disallow" {
                allowed = false;
            }
        }
    }
    allowed
}

pub(crate) fn resolve_native_classifier(
    natives: Option<&HashMap<String, String>>,
) -> Option<String> {
    let natives_map = natives?;
    let raw_classifier = natives_map.get(current_platform_name())?;
    let arch_suffix = if cfg!(target_pointer_width = "64") {
        "64"
    } else {
        "32"
    };
    Some(raw_classifier.replace("${arch}", arch_suffix))
}

/// Maven coordinate parser converting "group:artifact:version[:classifier]" into a relative path.
pub fn maven_coordinate_to_path(maven_id: &str) -> Option<String> {
    let parts: Vec<&str> = maven_id.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let classifier = if parts.len() >= 4 {
        format!("-{}", parts[3])
    } else {
        String::new()
    };
    Some(format!(
        "{}/{}/{}/{}-{}{}.jar",
        group, artifact, version, artifact, version, classifier
    ))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MojangArtifact {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub sha1: String,
    #[serde(default)]
    pub size: usize,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MojangRuleOs {
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MojangRule {
    pub action: String,
    pub os: Option<MojangRuleOs>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MojangDownloadInfo {
    pub artifact: Option<MojangArtifact>,
    #[serde(default)]
    pub classifiers: HashMap<String, MojangArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MojangLibrary {
    pub name: String,
    pub downloads: Option<MojangDownloadInfo>,
    #[serde(default)]
    pub rules: Vec<MojangRule>,
    pub natives: Option<HashMap<String, String>>,
}

impl MojangLibrary {
    pub fn is_allowed_on_current_os(&self) -> bool {
        evaluate_rules(&self.rules)
    }

    pub fn get_native_classifier(&self) -> Option<String> {
        resolve_native_classifier(self.natives.as_ref())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MojangClientDownload {
    pub sha1: String,
    pub size: usize,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MojangDownloads {
    pub client: Option<MojangClientDownload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MojangAssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: usize,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MojangVersionManifest {
    pub id: String,
    #[serde(rename = "assetIndex")]
    pub asset_index: Option<MojangAssetIndex>,
    pub downloads: Option<MojangDownloads>,
    #[serde(default)]
    pub libraries: Vec<MojangLibrary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrismLibraryDownloads {
    pub artifact: Option<MojangArtifact>,
    #[serde(default)]
    pub classifiers: HashMap<String, MojangArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrismLibrary {
    pub name: String,
    pub url: Option<String>,
    pub downloads: Option<PrismLibraryDownloads>,
    #[serde(default)]
    pub rules: Vec<MojangRule>,
    pub natives: Option<HashMap<String, String>>,
}

impl PrismLibrary {
    pub fn is_allowed_on_current_os(&self) -> bool {
        evaluate_rules(&self.rules)
    }

    pub fn get_native_classifier(&self) -> Option<String> {
        resolve_native_classifier(self.natives.as_ref())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrismComponentMeta {
    pub uid: Option<String>,
    pub version: Option<String>,
    #[serde(rename = "mainClass")]
    pub main_class: Option<String>,
    #[serde(rename = "+tweakers", default)]
    pub tweakers: Vec<String>,
    #[serde(rename = "+jvmArgs", default)]
    pub jvm_args: Vec<String>,
    #[serde(rename = "+gameArgs", default)]
    pub game_args: Vec<String>,
    #[serde(default)]
    pub libraries: Vec<PrismLibrary>,
}

pub async fn download_file_if_missing(
    url: &str,
    dest: &Path,
    expected_sha1: Option<&str>,
) -> Result<bool> {
    if dest.exists() {
        let is_valid = match expected_sha1 {
            Some(sha1) => crate::store::verify_file_hash(
                dest,
                sha1,
                crate::manifest::HashAlgorithm::Sha1,
            )?,
            None => true,
        };
        if is_valid {
            return Ok(false);
        }
    }

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|source| SzzError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let client = reqwest::Client::builder()
        .user_agent("szz-launcher/0.1.0")
        .build()
        .map_err(|source| SzzError::Network {
            url: url.to_string(),
            source,
        })?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|source| SzzError::Network {
            url: url.to_string(),
            source,
        })?;

    if !response.status().is_success() {
        return Err(SzzError::Loader(format!(
            "Failed to download {}: HTTP {}",
            url,
            response.status()
        )));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|source| SzzError::Network {
            url: url.to_string(),
            source,
        })?;

    if let Some(sha1) = expected_sha1 {
        let actual = hex::encode(sha1::Sha1::digest(&bytes));
        if !actual.eq_ignore_ascii_case(sha1) {
            return Err(SzzError::Loader(format!(
                "SHA1 mismatch downloading {}: expected {}, actual {}",
                url, sha1, actual
            )));
        }
    }

    let filename = dest
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let temp_nonce = TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let temp_file = dest.with_file_name(format!("{}.tmp-{}-{}", filename, std::process::id(), temp_nonce));
    std::fs::write(&temp_file, &bytes).map_err(|source| SzzError::Io {
        path: temp_file.clone(),
        source,
    })?;

    std::fs::rename(&temp_file, dest).map_err(|source| SzzError::Io {
        path: dest.to_path_buf(),
        source,
    })?;

    Ok(true)
}

pub async fn download_files_concurrently(
    tasks: Vec<(String, PathBuf, Option<String>)>,
) -> Result<()> {
    if tasks.is_empty() {
        return Ok(());
    }

    // Deduplicate tasks by destination path to prevent redundant downloads or collisions
    let mut seen = std::collections::HashSet::new();
    let mut unique_tasks = Vec::with_capacity(tasks.len());
    for task in tasks {
        if seen.insert(task.1.clone()) {
            unique_tasks.push(task);
        }
    }

    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(16));
    let mut join_set = tokio::task::JoinSet::new();

    for (url, dest, sha1) in unique_tasks {
        let sem = semaphore.clone();
        join_set.spawn(async move {
            let _permit = sem.acquire().await;
            download_file_if_missing(&url, &dest, sha1.as_deref()).await
        });
    }

    while let Some(res) = join_set.join_next().await {
        res.map_err(|e| SzzError::Loader(format!("Join error during library sync: {}", e)))??;
    }

    Ok(())
}

pub async fn fetch_mojang_version_manifest(
    paths: &SzzPaths,
    mc_version: &str,
) -> Result<MojangVersionManifest> {
    let version_dir = paths.versions_dir().join(mc_version);
    std::fs::create_dir_all(&version_dir).map_err(|source| SzzError::Io {
        path: version_dir.clone(),
        source,
    })?;

    let version_json_file = version_dir.join(format!("{}.json", mc_version));
    if version_json_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&version_json_file) {
            if let Ok(manifest) = serde_json::from_str::<MojangVersionManifest>(&content) {
                return Ok(manifest);
            }
        }
    }

    let index_url = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
    let client = reqwest::Client::builder()
        .user_agent("szz-launcher/0.1.0")
        .build()
        .map_err(|source| SzzError::Network {
            url: index_url.to_string(),
            source,
        })?;

    #[derive(Deserialize)]
    struct VersionEntry {
        id: String,
        url: String,
    }
    #[derive(Deserialize)]
    struct VersionIndex {
        versions: Vec<VersionEntry>,
    }

    let resp = client
        .get(index_url)
        .send()
        .await
        .map_err(|source| SzzError::Network {
            url: index_url.to_string(),
            source,
        })?
        .error_for_status()
        .map_err(|source| SzzError::Network {
            url: index_url.to_string(),
            source,
        })?;
    let index: VersionIndex = resp.json().await.map_err(|source| SzzError::Network {
        url: index_url.to_string(),
        source,
    })?;

    let package_url = index
        .versions
        .into_iter()
        .find(|v| v.id == mc_version)
        .map(|v| v.url)
        .ok_or_else(|| {
            SzzError::Loader(format!(
                "Minecraft version {} not found in Mojang version manifest",
                mc_version
            ))
        })?;

    let resp = client
        .get(&package_url)
        .send()
        .await
        .map_err(|source| SzzError::Network {
            url: package_url.clone(),
            source,
        })?
        .error_for_status()
        .map_err(|source| SzzError::Network {
            url: package_url.clone(),
            source,
        })?;

    let content = resp
        .text()
        .await
        .map_err(|source| SzzError::Network {
            url: package_url.clone(),
            source,
        })?;

    let manifest: MojangVersionManifest = serde_json::from_str(&content).map_err(|e| {
        SzzError::Loader(format!("Failed to parse Mojang version manifest: {}", e))
    })?;

    let temp_nonce = TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let temp_file = version_json_file.with_file_name(format!("{}.json.tmp-{}-{}", mc_version, std::process::id(), temp_nonce));
    if std::fs::write(&temp_file, &content).is_ok() {
        let _ = std::fs::rename(&temp_file, &version_json_file);
    }

    Ok(manifest)
}

pub async fn sync_vanilla_client_and_assets(
    paths: &SzzPaths,
    mc_version: &str,
    manifest: &MojangVersionManifest,
) -> Result<PathBuf> {
    let client_jar = paths.client_jar_path(mc_version);
    if !client_jar.exists() {
        if let Some(downloads) = &manifest.downloads {
            if let Some(client) = &downloads.client {
                download_file_if_missing(&client.url, &client_jar, Some(&client.sha1)).await?;
            } else {
                return Err(SzzError::Loader(format!(
                    "Client download not found in Mojang manifest for {}",
                    mc_version
                )));
            }
        } else {
            return Err(SzzError::Loader(format!(
                "Client download not found in Mojang manifest for {}",
                mc_version
            )));
        }
    }

    if let Some(asset_index) = &manifest.asset_index {
        let index_dir = paths.assets_dir().join("indexes");
        let id_file = index_dir.join(format!("{}.json", asset_index.id));
        let ver_file = index_dir.join(format!("{}.json", mc_version));
        download_file_if_missing(&asset_index.url, &id_file, Some(&asset_index.sha1)).await?;
        if !ver_file.exists() {
            let _ = std::fs::copy(&id_file, &ver_file);
        }
    }

    Ok(client_jar)
}

pub async fn sync_mojang_libraries_and_natives(
    paths: &SzzPaths,
    instance_id: Option<&str>,
    manifest: &MojangVersionManifest,
) -> Result<Vec<PathBuf>> {
    let mut mojang_lib_paths: Vec<PathBuf> = Vec::new();
    let mut download_tasks: Vec<(String, PathBuf, Option<String>)> = Vec::new();
    let mut native_jars_to_extract: Vec<PathBuf> = Vec::new();

    for lib in &manifest.libraries {
        if !lib.is_allowed_on_current_os() {
            continue;
        }

        if let Some(downloads) = &lib.downloads {
            // Standard library artifact
            if let Some(artifact) = &downloads.artifact {
                let dest = paths.libraries_dir().join(&artifact.path);
                if !dest.exists() {
                    download_tasks.push((
                        artifact.url.clone(),
                        dest.clone(),
                        Some(artifact.sha1.clone()),
                    ));
                }
                mojang_lib_paths.push(dest);
            }

            // Native classifiers
            if instance_id.is_some() {
                if let Some(classifier_key) = lib.get_native_classifier() {
                    if let Some(native_artifact) = downloads.classifiers.get(&classifier_key) {
                        let dest = paths.libraries_dir().join(&native_artifact.path);
                        if !dest.exists() {
                            download_tasks.push((
                                native_artifact.url.clone(),
                                dest.clone(),
                                Some(native_artifact.sha1.clone()),
                            ));
                        }
                        native_jars_to_extract.push(dest);
                    }
                }
            }
        }
    }

    if !download_tasks.is_empty() {
        println!(
            "[szz-core] Synchronizing {} Mojang libraries concurrently...",
            download_tasks.len()
        );
        download_files_concurrently(download_tasks).await?;
    }

    // Extract native libraries if an instance_id was provided
    if let Some(instance) = instance_id {
        let natives_dir = paths.instance_natives_dir(instance);
        std::fs::create_dir_all(&natives_dir).map_err(|source| SzzError::Io {
            path: natives_dir.clone(),
            source,
        })?;

        if !native_jars_to_extract.is_empty() {
            extract_natives(&natives_dir, &native_jars_to_extract)?;
        }
    }

    Ok(mojang_lib_paths)
}

pub async fn sync_prism_libraries_and_natives(
    paths: &SzzPaths,
    instance_id: Option<&str>,
    libraries: &[PrismLibrary],
    loader_name: &str,
) -> Result<Vec<PathBuf>> {
    let mut lib_paths: Vec<PathBuf> = Vec::new();
    let mut download_tasks: Vec<(String, PathBuf, Option<String>)> = Vec::new();
    let mut native_jars_to_extract: Vec<PathBuf> = Vec::new();

    let add_maven_fallback = |lib: &PrismLibrary, rel_path: &str, lib_paths: &mut Vec<PathBuf>, download_tasks: &mut Vec<(String, PathBuf, Option<String>)>| {
        let dest = paths.libraries_dir().join(rel_path);
        let url = match &lib.url {
            Some(base_url) => format!("{}/{}", base_url.trim_end_matches('/'), rel_path),
            None => format!("https://libraries.minecraft.net/{}", rel_path),
        };
        if !dest.exists() {
            download_tasks.push((url, dest.clone(), None));
        }
        lib_paths.push(dest);
    };

    for lib in libraries {
        if !lib.is_allowed_on_current_os() {
            continue;
        }

        if let Some(downloads) = &lib.downloads {
            if let Some(artifact) = &downloads.artifact {
                let dest = paths.libraries_dir().join(&artifact.path);
                if !dest.exists() {
                    download_tasks.push((
                        artifact.url.clone(),
                        dest.clone(),
                        if artifact.sha1.is_empty() {
                            None
                        } else {
                            Some(artifact.sha1.clone())
                        },
                    ));
                }
                lib_paths.push(dest);
            } else if let Some(rel_path) = maven_coordinate_to_path(&lib.name) {
                add_maven_fallback(lib, &rel_path, &mut lib_paths, &mut download_tasks);
            }

            if instance_id.is_some() {
                if let Some(classifier_key) = lib.get_native_classifier() {
                    if let Some(native_artifact) = downloads.classifiers.get(&classifier_key) {
                        let dest = paths.libraries_dir().join(&native_artifact.path);
                        if !dest.exists() {
                            download_tasks.push((
                                native_artifact.url.clone(),
                                dest.clone(),
                                if native_artifact.sha1.is_empty() {
                                    None
                                } else {
                                    Some(native_artifact.sha1.clone())
                                },
                            ));
                        }
                        native_jars_to_extract.push(dest);
                    }
                }
            }
        } else if let Some(rel_path) = maven_coordinate_to_path(&lib.name) {
            add_maven_fallback(lib, &rel_path, &mut lib_paths, &mut download_tasks);
        }
    }

    if !download_tasks.is_empty() {
        println!(
            "[szz-core] Synchronizing {} {} libraries concurrently...",
            download_tasks.len(),
            loader_name
        );
        download_files_concurrently(download_tasks).await?;
    }

    if let Some(instance) = instance_id {
        if !native_jars_to_extract.is_empty() {
            let natives_dir = paths.instance_natives_dir(instance);
            std::fs::create_dir_all(&natives_dir).map_err(|source| SzzError::Io {
                path: natives_dir.clone(),
                source,
            })?;

            extract_natives(&natives_dir, &native_jars_to_extract)?;
        }
    }

    Ok(lib_paths)
}

pub async fn sync_vanilla_and_assemble_classpath(
    paths: &SzzPaths,
    instance_id: &str,
    mc_version: &str,
) -> Result<ResolvedLaunchMeta> {
    paths.ensure_dirs()?;
    let mojang_manifest = fetch_mojang_version_manifest(paths, mc_version).await?;

    let (client_jar, mojang_libs) = tokio::try_join!(
        sync_vanilla_client_and_assets(paths, mc_version, &mojang_manifest),
        sync_mojang_libraries_and_natives(paths, Some(instance_id), &mojang_manifest),
    )?;

    let mut all_classpath = mojang_libs;
    all_classpath.push(client_jar);

    let classpath = std::env::join_paths(&all_classpath)
        .map_err(|e| SzzError::Loader(format!("Classpath joining failed: {}", e)))?
        .to_string_lossy()
        .to_string();

    Ok(ResolvedLaunchMeta::new(
        classpath,
        "net.minecraft.client.main.Main".to_string(),
        all_classpath.len(),
    ))
}

fn normalize_component_version<'a>(component_id: &str, version: &'a str) -> &'a str {
    if component_id == "net.minecraftforge" || component_id == "net.neoforged" {
        if let Some((_mc, forge_ver)) = version.split_once('-') {
            if forge_ver.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                return forge_ver;
            }
        }
    }
    version
}

pub async fn fetch_prism_meta(
    paths: &SzzPaths,
    component_id: &str,
    version: &str,
) -> Result<PrismComponentMeta> {
    let cache_dir = paths.meta_cache_dir().join(component_id);
    std::fs::create_dir_all(&cache_dir).map_err(|source| SzzError::Io {
        path: cache_dir.clone(),
        source,
    })?;

    let normalized_version = normalize_component_version(component_id, version);

    if normalized_version.contains('/') || normalized_version.contains('\\') || normalized_version.contains("..") {
        return Err(SzzError::Security(format!(
            "Invalid version string for {}: {}",
            component_id, normalized_version
        )));
    }

    let cache_file = cache_dir.join(format!("{}.json", normalized_version));
    if !cache_file.starts_with(&cache_dir) {
        return Err(SzzError::Security(format!(
            "Cache file path traversal out of bounds: {:?}",
            cache_file
        )));
    }
    if cache_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&cache_file) {
            if let Ok(meta) = serde_json::from_str::<PrismComponentMeta>(&content) {
                return Ok(meta);
            }
        }
    }

    let base_url = std::env::var("SZZ_PRISM_META_URL")
        .unwrap_or_else(|_| "https://meta.prismlauncher.org/v1".to_string());
    let url = format!("{}/{}/{}.json", base_url.trim_end_matches('/'), component_id, normalized_version);

    let client = reqwest::Client::builder()
        .user_agent("szz-launcher/0.1.0")
        .build()
        .map_err(|source| SzzError::Network {
            url: url.clone(),
            source,
        })?;

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|source| SzzError::Network {
            url: url.clone(),
            source,
        })?;

    if !resp.status().is_success() {
        return Err(SzzError::Loader(format!(
            "PrismMeta API returned HTTP {} for component {} version {}",
            resp.status(),
            component_id,
            normalized_version
        )));
    }

    let content = resp
        .text()
        .await
        .map_err(|source| SzzError::Network {
            url: url.clone(),
            source,
        })?;

    let meta: PrismComponentMeta = serde_json::from_str(&content).map_err(|e| {
        SzzError::Loader(format!(
            "Failed to parse PrismMeta JSON for {} {}: {}",
            component_id, normalized_version, e
        ))
    })?;

    let temp_nonce = TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let temp_file = cache_file.with_file_name(format!("{}.tmp-{}-{}", normalized_version, std::process::id(), temp_nonce));
    if let Ok(()) = std::fs::write(&temp_file, &content) {
        let _ = std::fs::rename(&temp_file, &cache_file);
    }

    Ok(meta)
}
