use crate::error::{Result, SzzError};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SzzPaths {
    pub root: PathBuf,
}

impl SzzPaths {
    pub fn new() -> Result<Self> {
        if let Some(val) = std::env::var("SZZ_HOME").ok().filter(|v| !v.trim().is_empty()) {
            return Ok(Self::custom(val));
        }
        let home = directories::BaseDirs::new()
            .ok_or_else(|| SzzError::Runtime("Failed to resolve user home directory".to_string()))?;
        let root = home.home_dir().join(".szz");
        Ok(Self { root })
    }

    pub fn custom(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn store_dir(&self) -> PathBuf {
        self.root.join("store")
    }

    pub fn runtimes_dir(&self) -> PathBuf {
        self.root.join("runtimes")
    }

    pub fn java_runtime_dir(&self, major_version: u32) -> PathBuf {
        self.runtimes_dir().join(format!("java-{}", major_version))
    }

    pub fn java_binary_path(&self, major_version: u32) -> PathBuf {
        let runtime = self.java_runtime_dir(major_version);
        #[cfg(target_os = "windows")]
        {
            runtime.join("bin").join("javaw.exe")
        }
        #[cfg(not(target_os = "windows"))]
        {
            runtime.join("bin").join("java")
        }
    }

    pub fn versions_dir(&self) -> PathBuf {
        self.root.join("versions")
    }

    pub fn version_dir(&self, mc_version: &str) -> PathBuf {
        self.versions_dir().join(mc_version)
    }

    pub fn client_jar_path(&self, mc_version: &str) -> PathBuf {
        self.version_dir(mc_version).join(format!("{}.jar", mc_version))
    }

    pub fn version_json_path(&self, mc_version: &str) -> PathBuf {
        self.version_dir(mc_version).join(format!("{}.json", mc_version))
    }

    pub fn libraries_dir(&self) -> PathBuf {
        self.root.join("libraries")
    }

    pub fn assets_dir(&self) -> PathBuf {
        self.root.join("assets")
    }

    pub fn assets_indexes_dir(&self) -> PathBuf {
        self.assets_dir().join("indexes")
    }

    pub fn assets_objects_dir(&self) -> PathBuf {
        self.assets_dir().join("objects")
    }

    pub fn asset_index_path(&self, id: &str) -> PathBuf {
        self.assets_indexes_dir().join(format!("{}.json", id))
    }

    pub fn asset_object_path(&self, hash: &str) -> PathBuf {
        let prefix = if hash.len() >= 2 { &hash[0..2] } else { "xx" };
        self.assets_objects_dir().join(prefix).join(hash)
    }

    pub fn instances_dir(&self) -> PathBuf {
        self.root.join("instances")
    }

    pub fn instance_dir(&self, instance_id: &str) -> PathBuf {
        self.instances_dir().join(instance_id)
    }

    pub fn instance_mods_dir(&self, instance_id: &str) -> PathBuf {
        self.instance_dir(instance_id).join("mods")
    }

    pub fn instance_natives_dir(&self, instance_id: &str) -> PathBuf {
        self.instance_dir(instance_id).join("natives")
    }

    pub fn instance_pid_file(&self, instance_id: &str) -> PathBuf {
        self.instance_dir(instance_id).join(".szz_instance.pid")
    }

    pub fn instance_metadata_file(&self, instance_id: &str) -> PathBuf {
        self.instance_dir(instance_id).join(".szz_instance.json")
    }

    pub fn meta_cache_dir(&self) -> PathBuf {
        self.root.join("cache").join("meta")
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.root.join("cache")
    }

    pub fn catalog_cache_file(&self) -> PathBuf {
        self.cache_dir().join("catalog.json")
    }

    pub fn catalog_meta_file(&self) -> PathBuf {
        self.cache_dir().join("catalog.meta.json")
    }

    pub fn accounts_file(&self) -> PathBuf {
        self.root.join("accounts.json")
    }

    pub fn settings_file(&self) -> PathBuf {
        self.root.join("settings.json")
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        let dirs = [
            &self.root,
            &self.store_dir(),
            &self.runtimes_dir(),
            &self.versions_dir(),
            &self.libraries_dir(),
            &self.assets_dir(),
            &self.assets_indexes_dir(),
            &self.assets_objects_dir(),
            &self.instances_dir(),
            &self.meta_cache_dir(),
        ];

        for dir in dirs {
            std::fs::create_dir_all(dir).map_err(|source| SzzError::Io {
                path: dir.to_path_buf(),
                source,
            })?;
        }

        Ok(())
    }

    pub fn ensure_instance_dirs(&self, instance_id: &str) -> Result<PathBuf> {
        crate::instance::validate_instance_id(instance_id)?;
        let instance = self.instance_dir(instance_id);
        let mods = self.instance_mods_dir(instance_id);
        let natives = self.instance_natives_dir(instance_id);
        std::fs::create_dir_all(&mods).map_err(|source| SzzError::Io {
            path: mods.clone(),
            source,
        })?;
        std::fs::create_dir_all(&natives).map_err(|source| SzzError::Io {
            path: natives.clone(),
            source,
        })?;
        Ok(instance)
    }
}
