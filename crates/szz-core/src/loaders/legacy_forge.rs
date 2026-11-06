use crate::error::{Result, SzzError};
use crate::loaders::common::{
    fetch_mojang_version_manifest, fetch_prism_meta, sync_mojang_libraries_and_natives,
    sync_prism_libraries_and_natives, sync_vanilla_client_and_assets,
};
use crate::loaders::ResolvedLaunchMeta;
use crate::paths::SzzPaths;
use std::path::PathBuf;

/// Synchronizes Legacy Forge (<= 1.12.2) libraries, Mojang vanilla assets/libraries,
/// extracts LWJGL 2 native binaries, and assembles the launch classpath and arguments.
pub async fn sync_legacy_forge_and_assemble_classpath(
    paths: &SzzPaths,
    instance_id: &str,
    mc_version: &str,
    loader_version: &str,
) -> Result<ResolvedLaunchMeta> {
    paths.ensure_dirs()?;

    let mojang_manifest = fetch_mojang_version_manifest(paths, mc_version).await?;

    let (client_jar, mojang_libs) = tokio::try_join!(
        sync_vanilla_client_and_assets(paths, mc_version, &mojang_manifest),
        sync_mojang_libraries_and_natives(paths, Some(instance_id), &mojang_manifest),
    )?;

    let forge_meta = fetch_prism_meta(paths, "net.minecraftforge", loader_version).await?;

    let forge_lib_paths = sync_prism_libraries_and_natives(
        paths,
        Some(instance_id),
        &forge_meta.libraries,
        "Legacy Forge",
    )
    .await?;

    let mut all_classpath: Vec<PathBuf> =
        Vec::with_capacity(forge_lib_paths.len() + mojang_libs.len() + 1);
    all_classpath.extend(forge_lib_paths);
    all_classpath.extend(mojang_libs);
    all_classpath.push(client_jar);

    let classpath_str = std::env::join_paths(&all_classpath)
        .map_err(|e| SzzError::Loader(format!("Classpath joining failed: {}", e)))?
        .to_string_lossy()
        .to_string();

    let main_class = forge_meta
        .main_class
        .unwrap_or_else(|| "net.minecraft.launchwrapper.Launch".to_string());

    let mut extra_game_args = Vec::new();
    if !forge_meta.tweakers.is_empty() {
        for tweaker in &forge_meta.tweakers {
            extra_game_args.push("--tweakClass".to_string());
            extra_game_args.push(tweaker.clone());
        }
    } else {
        extra_game_args.push("--tweakClass".to_string());
        extra_game_args.push("net.minecraftforge.fml.common.launcher.FMLTweaker".to_string());
    }
    extra_game_args.extend(forge_meta.game_args);

    let extra_jvm_args = forge_meta.jvm_args;

    Ok(ResolvedLaunchMeta {
        classpath: classpath_str,
        main_class,
        library_count: all_classpath.len(),
        extra_jvm_args,
        extra_game_args,
    })
}
