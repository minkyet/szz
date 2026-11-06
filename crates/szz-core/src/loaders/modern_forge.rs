use crate::error::{Result, SzzError};
use crate::loaders::common::{
    fetch_mojang_version_manifest, fetch_prism_meta, sync_mojang_libraries_and_natives,
    sync_prism_libraries_and_natives, sync_vanilla_client_and_assets,
};
use crate::loaders::natives::is_version_at_least;
use crate::loaders::ResolvedLaunchMeta;
use crate::paths::SzzPaths;
use std::collections::HashMap;
use std::path::PathBuf;

/// Substitutes `${key}` template placeholders with corresponding values.
pub fn substitute_template_vars(arg: &str, vars: &HashMap<&str, &str>) -> String {
    if !arg.contains("${") {
        return arg.to_string();
    }
    let mut result = arg.to_string();
    for (k, v) in vars {
        let placeholder = format!("${{{}}}", k);
        result = result.replace(&placeholder, v);
    }
    result
}

/// Synchronizes Modern Forge (1.13+) or NeoForge libraries, Mojang vanilla assets/libraries,
/// handles LWJGL 3 natives, performs ModLauncher argument template substitution,
/// and applies the Minecraft 1.17+ classpath module isolation rule (excluding client.jar from -cp).
pub async fn sync_modern_forge_and_assemble_classpath(
    paths: &SzzPaths,
    instance_id: &str,
    mc_version: &str,
    loader_version: &str,
    is_neoforge: bool,
) -> Result<ResolvedLaunchMeta> {
    paths.ensure_dirs()?;

    let mojang_manifest = fetch_mojang_version_manifest(paths, mc_version).await?;

    let (client_jar, mojang_libs) = tokio::try_join!(
        sync_vanilla_client_and_assets(paths, mc_version, &mojang_manifest),
        sync_mojang_libraries_and_natives(paths, Some(instance_id), &mojang_manifest),
    )?;

    let component_id = if is_neoforge {
        "net.neoforged"
    } else {
        "net.minecraftforge"
    };
    let forge_meta = fetch_prism_meta(paths, component_id, loader_version).await?;

    let loader_name = if is_neoforge {
        "NeoForge"
    } else {
        "Modern Forge"
    };
    let loader_lib_paths = sync_prism_libraries_and_natives(
        paths,
        Some(instance_id),
        &forge_meta.libraries,
        loader_name,
    )
    .await?;

    // Minecraft 1.17+ ModLauncher module isolation rule: client.jar is excluded from -cp
    let is_mc_117_or_above = is_version_at_least(mc_version, "1.17");
    let mut all_classpath: Vec<PathBuf> =
        Vec::with_capacity(loader_lib_paths.len() + mojang_libs.len() + 1);
    all_classpath.extend(loader_lib_paths);
    all_classpath.extend(mojang_libs);

    if !is_mc_117_or_above {
        all_classpath.push(client_jar);
    }

    let classpath_str = std::env::join_paths(&all_classpath)
        .map_err(|e| SzzError::Loader(format!("Classpath joining failed: {}", e)))?
        .to_string_lossy()
        .to_string();

    let main_class = forge_meta
        .main_class
        .unwrap_or_else(|| "io.github.zekerzhayard.forgewrapper.installer.Main".to_string());

    let lib_dir_str = paths.libraries_dir().to_string_lossy().to_string();
    let natives_dir_str = paths.instance_natives_dir(instance_id).to_string_lossy().to_string();
    let game_dir_str = paths.instance_dir(instance_id).to_string_lossy().to_string();
    let assets_dir_str = paths.assets_dir().to_string_lossy().to_string();
    let cp_sep = if cfg!(windows) { ";" } else { ":" };

    let mut vars: HashMap<&str, &str> = HashMap::new();
    vars.insert("library_directory", &lib_dir_str);
    vars.insert("classpath_separator", cp_sep);
    vars.insert("version_name", mc_version);
    vars.insert("natives_directory", &natives_dir_str);
    vars.insert("game_directory", &game_dir_str);
    vars.insert("assets_root", &assets_dir_str);
    vars.insert("assets_index_name", mc_version);
    vars.insert("launcher_name", "szz");
    vars.insert("launcher_version", "0.1.0");

    let mut extra_game_args = Vec::new();
    for tweaker in &forge_meta.tweakers {
        extra_game_args.push("--tweakClass".to_string());
        extra_game_args.push(substitute_template_vars(tweaker, &vars));
    }
    for arg in &forge_meta.game_args {
        extra_game_args.push(substitute_template_vars(arg, &vars));
    }

    let extra_jvm_args: Vec<String> = forge_meta
        .jvm_args
        .iter()
        .map(|arg| substitute_template_vars(arg, &vars))
        .collect();

    Ok(ResolvedLaunchMeta {
        classpath: classpath_str,
        main_class,
        library_count: all_classpath.len(),
        extra_jvm_args,
        extra_game_args,
    })
}
