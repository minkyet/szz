use crate::error::{Result, SzzError};
use crate::loaders::common::{
    fetch_mojang_version_manifest, fetch_prism_meta, sync_mojang_libraries_and_natives,
    sync_prism_libraries_and_natives, sync_vanilla_client_and_assets,
};
use crate::loaders::ResolvedLaunchMeta;
use crate::paths::SzzPaths;
use std::path::PathBuf;

/// Synchronizes LiteLoader libraries, Mojang vanilla assets/libraries,
/// extracts LWJGL natives, and assembles the Launchwrapper classpath and LiteLoaderTweaker arguments.
pub async fn sync_liteloader_and_assemble_classpath(
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

    let lite_meta = fetch_prism_meta(paths, "com.mumfrey.liteloader", loader_version).await?;

    let lite_lib_paths = sync_prism_libraries_and_natives(
        paths,
        Some(instance_id),
        &lite_meta.libraries,
        "LiteLoader",
    )
    .await?;

    let mut all_classpath: Vec<PathBuf> =
        Vec::with_capacity(lite_lib_paths.len() + mojang_libs.len() + 1);
    all_classpath.extend(lite_lib_paths);
    all_classpath.extend(mojang_libs);
    all_classpath.push(client_jar);

    let classpath_str = std::env::join_paths(&all_classpath)
        .map_err(|e| SzzError::Loader(format!("Classpath joining failed: {}", e)))?
        .to_string_lossy()
        .to_string();

    let main_class = lite_meta
        .main_class
        .unwrap_or_else(|| "net.minecraft.launchwrapper.Launch".to_string());

    let mut extra_game_args = Vec::new();
    if !lite_meta.tweakers.is_empty() {
        for tweaker in &lite_meta.tweakers {
            extra_game_args.push("--tweakClass".to_string());
            extra_game_args.push(tweaker.clone());
        }
    } else {
        extra_game_args.push("--tweakClass".to_string());
        extra_game_args.push("com.mumfrey.liteloader.launch.LiteLoaderTweaker".to_string());
    }
    extra_game_args.extend(lite_meta.game_args);

    let extra_jvm_args = lite_meta.jvm_args;

    Ok(ResolvedLaunchMeta {
        classpath: classpath_str,
        main_class,
        library_count: all_classpath.len(),
        extra_jvm_args,
        extra_game_args,
    })
}
