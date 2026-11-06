use crate::error::{Result, SzzError};
use crate::loaders::common;
use crate::loaders::ResolvedLaunchMeta;
use crate::paths::SzzPaths;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub type ResolvedClasspath = ResolvedLaunchMeta;

// Re-export common Mojang & library utilities for backward compatibility
pub use crate::loaders::common::{
    download_files_concurrently, fetch_mojang_version_manifest, maven_coordinate_to_path,
    MojangArtifact, MojangAssetIndex, MojangClientDownload, MojangDownloadInfo, MojangDownloads,
    MojangLibrary, MojangRule, MojangRuleOs, MojangVersionManifest,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricMavenArtifact {
    pub maven: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricLibrary {
    pub name: String,
    pub url: String,
    pub sha1: Option<String>,
    pub size: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricLibraries {
    #[serde(default)]
    pub client: Vec<FabricLibrary>,
    #[serde(default)]
    pub common: Vec<FabricLibrary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricMainClass {
    pub client: String,
    pub server: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricLauncherMeta {
    pub version: u32,
    pub libraries: FabricLibraries,
    #[serde(rename = "mainClass")]
    pub main_class: FabricMainClass,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricLoaderResponse {
    pub loader: FabricMavenArtifact,
    pub intermediary: FabricMavenArtifact,
    #[serde(rename = "launcherMeta")]
    pub launcher_meta: FabricLauncherMeta,
}

pub async fn fetch_fabric_metadata(
    mc_version: &str,
    loader_version: &str,
) -> Result<FabricLoaderResponse> {
    let url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}/{}",
        mc_version, loader_version
    );

    let client = reqwest::Client::builder()
        .user_agent("szz-launcher/0.1.0")
        .build()
        .map_err(|source| SzzError::Network {
            url: url.clone(),
            source,
        })?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|source| SzzError::Network {
            url: url.clone(),
            source,
        })?;

    if !response.status().is_success() {
        return Err(SzzError::Fabric(format!(
            "Fabric meta API returned HTTP {} for {}",
            response.status(),
            url
        )));
    }

    let meta: FabricLoaderResponse = response.json().await.map_err(|source| SzzError::Network {
        url,
        source,
    })?;

    Ok(meta)
}

/// Synchronizes vanilla Minecraft client jar, Mojang libraries, and Fabric libraries,
/// assembling the complete, ready-to-launch classpath.
pub async fn sync_fabric_and_assemble_classpath(
    paths: &SzzPaths,
    mc_version: &str,
    loader_version: &str,
) -> Result<ResolvedClasspath> {
    paths.ensure_dirs()?;

    let mojang_manifest = common::fetch_mojang_version_manifest(paths, mc_version).await?;

    let (client_jar, mojang_libs) = tokio::try_join!(
        common::sync_vanilla_client_and_assets(paths, mc_version, &mojang_manifest),
        common::sync_mojang_libraries_and_natives(paths, None, &mojang_manifest),
    )?;

    let fabric_meta = fetch_fabric_metadata(mc_version, loader_version).await?;

    let mut fabric_lib_paths: Vec<PathBuf> = Vec::new();
    let mut download_tasks: Vec<(String, PathBuf, Option<String>)> = Vec::new();

    for artifact in [&fabric_meta.loader.maven, &fabric_meta.intermediary.maven] {
        if let Some(rel_path) = maven_coordinate_to_path(artifact) {
            let dest = paths.libraries_dir().join(&rel_path);
            let url = format!("https://maven.fabricmc.net/{}", rel_path);
            if !dest.exists() {
                download_tasks.push((url, dest.clone(), None));
            }
            fabric_lib_paths.push(dest);
        }
    }

    let all_fabric_libs = fabric_meta
        .launcher_meta
        .libraries
        .common
        .iter()
        .chain(fabric_meta.launcher_meta.libraries.client.iter());

    for lib in all_fabric_libs {
        if let Some(rel_path) = maven_coordinate_to_path(&lib.name) {
            let dest = paths.libraries_dir().join(&rel_path);
            let url = format!("{}/{}", lib.url.trim_end_matches('/'), rel_path);
            if !dest.exists() {
                download_tasks.push((url, dest.clone(), lib.sha1.clone()));
            }
            fabric_lib_paths.push(dest);
        }
    }

    if !download_tasks.is_empty() {
        println!(
            "[szz-core] Synchronizing {} Fabric libraries concurrently...",
            download_tasks.len()
        );
        download_files_concurrently(download_tasks).await?;
    }

    let mut all_classpath_paths: Vec<PathBuf> =
        Vec::with_capacity(fabric_lib_paths.len() + mojang_libs.len() + 1);
    all_classpath_paths.extend(fabric_lib_paths);
    all_classpath_paths.extend(mojang_libs);
    all_classpath_paths.push(client_jar);

    let classpath_str = std::env::join_paths(&all_classpath_paths)
        .map_err(|e| SzzError::Fabric(format!("Classpath joining failed: {}", e)))?
        .to_string_lossy()
        .to_string();

    Ok(ResolvedLaunchMeta {
        classpath: classpath_str,
        main_class: fabric_meta.launcher_meta.main_class.client,
        library_count: all_classpath_paths.len(),
        extra_jvm_args: Vec::new(),
        extra_game_args: Vec::new(),
    })
}
