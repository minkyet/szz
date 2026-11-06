use crate::error::Result;

pub mod common;
pub mod fabric;
pub mod legacy_forge;
pub mod liteloader;
pub mod modern_forge;
pub mod natives;

pub use common::{
    download_file_if_missing, download_files_concurrently, fetch_mojang_version_manifest,
    fetch_prism_meta, maven_coordinate_to_path, MojangArtifact, MojangLibrary, MojangRule,
    MojangVersionManifest, PrismComponentMeta, PrismLibrary,
};
pub use fabric::sync_fabric_and_assemble_classpath;
pub use legacy_forge::sync_legacy_forge_and_assemble_classpath;
pub use liteloader::sync_liteloader_and_assemble_classpath;
pub use modern_forge::{substitute_template_vars, sync_modern_forge_and_assemble_classpath};
pub use natives::{extract_natives, is_version_at_least, parse_server_address};

#[derive(Debug, Clone)]
pub struct ResolvedLaunchMeta {
    pub classpath: String,
    pub main_class: String,
    pub library_count: usize,
    pub extra_jvm_args: Vec<String>,
    pub extra_game_args: Vec<String>,
}

pub type ResolvedClasspath = ResolvedLaunchMeta;

impl ResolvedLaunchMeta {
    pub fn new(classpath: String, main_class: String, library_count: usize) -> Self {
        Self {
            classpath,
            main_class,
            library_count,
            extra_jvm_args: Vec::new(),
            extra_game_args: Vec::new(),
        }
    }
}

/// Dispatches metadata resolution, library downloading, native extraction,
/// and classpath assembly based on the server manifest's loader configuration.
pub async fn resolve_loader(
    paths: &crate::paths::SzzPaths,
    instance_id: &str,
    manifest: &crate::manifest::ServerManifest,
) -> Result<ResolvedLaunchMeta> {
    use crate::manifest::LoaderType;

    match manifest.loader.loader_type {
        LoaderType::Fabric => {
            sync_fabric_and_assemble_classpath(
                paths,
                &manifest.minecraft_version,
                &manifest.loader.version,
            )
            .await
        }
        LoaderType::Forge => {
            // Split Forge between Legacy (<= 1.12.2) and Modern (>= 1.13)
            if is_version_at_least(&manifest.minecraft_version, "1.13") {
                sync_modern_forge_and_assemble_classpath(
                    paths,
                    instance_id,
                    &manifest.minecraft_version,
                    &manifest.loader.version,
                    false,
                )
                .await
            } else {
                sync_legacy_forge_and_assemble_classpath(
                    paths,
                    instance_id,
                    &manifest.minecraft_version,
                    &manifest.loader.version,
                )
                .await
            }
        }
        LoaderType::NeoForge => {
            sync_modern_forge_and_assemble_classpath(
                paths,
                instance_id,
                &manifest.minecraft_version,
                &manifest.loader.version,
                true,
            )
            .await
        }
        LoaderType::LiteLoader => {
            sync_liteloader_and_assemble_classpath(
                paths,
                instance_id,
                &manifest.minecraft_version,
                &manifest.loader.version,
            )
            .await
        }
        LoaderType::Vanilla => {
            common::sync_vanilla_and_assemble_classpath(
                paths,
                instance_id,
                &manifest.minecraft_version,
            )
            .await
        }
    }
}
