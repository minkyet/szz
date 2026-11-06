use crate::error::{Result, SzzError};
use crate::paths::SzzPaths;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub const MIN_MEMORY_MB: u32 = 1024;
pub const DEFAULT_MEMORY_MB: u32 = 4096;
pub const DEFAULT_RESOLUTION_WIDTH: u32 = 1280;
pub const DEFAULT_RESOLUTION_HEIGHT: u32 = 720;

fn default_resolution_width() -> u32 {
    DEFAULT_RESOLUTION_WIDTH
}

fn default_resolution_height() -> u32 {
    DEFAULT_RESOLUTION_HEIGHT
}

fn default_true() -> bool {
    true
}

fn default_allocated_memory_mb() -> u32 {
    DEFAULT_MEMORY_MB
}

fn default_theme() -> String {
    "latte".to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LauncherBehaviorOnLaunch {
    #[default]
    #[serde(alias = "keepOpen", alias = "KeepOpen")]
    KeepOpen,
    #[serde(alias = "minimize", alias = "Minimize")]
    Minimize,
    #[serde(alias = "close", alias = "Close")]
    Close,
}

impl LauncherBehaviorOnLaunch {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::KeepOpen => "keep_open",
            Self::Minimize => "minimize",
            Self::Close => "close",
        }
    }

    pub fn from_str_value(val: &str) -> Option<Self> {
        match val.trim().to_ascii_lowercase().as_str() {
            "keep_open" | "keepopen" | "keep-open" => Some(Self::KeepOpen),
            "minimize" => Some(Self::Minimize),
            "close" => Some(Self::Close),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameSettings {
    #[serde(default = "default_resolution_width", alias = "resolution_width")]
    pub resolution_width: u32,
    #[serde(default = "default_resolution_height", alias = "resolution_height")]
    pub resolution_height: u32,
    #[serde(default)]
    pub fullscreen: bool,
    #[serde(default = "default_true", alias = "auto_connect")]
    pub auto_connect: bool,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            resolution_width: DEFAULT_RESOLUTION_WIDTH,
            resolution_height: DEFAULT_RESOLUTION_HEIGHT,
            fullscreen: false,
            auto_connect: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaSettings {
    #[serde(default = "default_allocated_memory_mb", alias = "allocated_memory_mb")]
    pub allocated_memory_mb: u32,
    #[serde(default, alias = "custom_jvm_args")]
    pub custom_jvm_args: Vec<String>,
}

impl Default for JavaSettings {
    fn default() -> Self {
        Self {
            allocated_memory_mb: DEFAULT_MEMORY_MB,
            custom_jvm_args: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherSettings {
    #[serde(default, alias = "behavior_on_launch")]
    pub behavior_on_launch: LauncherBehaviorOnLaunch,
    #[serde(default = "default_theme")]
    pub theme: String,
}

impl Default for LauncherSettings {
    fn default() -> Self {
        Self {
            behavior_on_launch: LauncherBehaviorOnLaunch::KeepOpen,
            theme: default_theme(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UserSettings {
    #[serde(default)]
    pub game: GameSettings,
    #[serde(default)]
    pub java: JavaSettings,
    #[serde(default)]
    pub launcher: LauncherSettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemMemoryInfo {
    pub total_memory_mb: u64,
    pub available_memory_mb: u64,
}

pub fn get_system_memory_info() -> SystemMemoryInfo {
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
            let mut total_kb: Option<u64> = None;
            let mut avail_kb: Option<u64> = None;
            let mut free_kb: Option<u64> = None;

            for line in content.lines() {
                if let Some(rest) = line.strip_prefix("MemTotal:") {
                    total_kb = rest
                        .split_whitespace()
                        .next()
                        .and_then(|v| v.parse::<u64>().ok());
                } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
                    avail_kb = rest
                        .split_whitespace()
                        .next()
                        .and_then(|v| v.parse::<u64>().ok());
                } else if let Some(rest) = line.strip_prefix("MemFree:") {
                    free_kb = rest
                        .split_whitespace()
                        .next()
                        .and_then(|v| v.parse::<u64>().ok());
                }
                if total_kb.is_some() && avail_kb.is_some() {
                    break;
                }
            }

            if let Some(total) = total_kb {
                let avail = avail_kb.or(free_kb).unwrap_or(total / 2);
                return SystemMemoryInfo {
                    total_memory_mb: (total / 1024).max(2048),
                    available_memory_mb: (avail / 1024).max(512),
                };
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        unsafe {
            let pages = libc::sysconf(libc::_SC_PHYS_PAGES);
            let page_size = libc::sysconf(libc::_SC_PAGE_SIZE);
            if pages > 0 && page_size > 0 {
                let total_bytes = (pages as u64).saturating_mul(page_size as u64);
                let total_mb = (total_bytes / (1024 * 1024)).max(2048);
                return SystemMemoryInfo {
                    total_memory_mb: total_mb,
                    available_memory_mb: total_mb / 2,
                };
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        #[repr(C)]
        struct MemoryStatusEx {
            dw_length: u32,
            dw_memory_load: u32,
            ull_total_phys: u64,
            ull_avail_phys: u64,
            ull_total_page_file: u64,
            ull_avail_page_file: u64,
            ull_total_virtual: u64,
            ull_avail_virtual: u64,
            ull_avail_extended_virtual: u64,
        }

        extern "system" {
            fn GlobalMemoryStatusEx(lp_buffer: *mut MemoryStatusEx) -> i32;
        }

        unsafe {
            let mut status = MemoryStatusEx {
                dw_length: std::mem::size_of::<MemoryStatusEx>() as u32,
                dw_memory_load: 0,
                ull_total_phys: 0,
                ull_avail_phys: 0,
                ull_total_page_file: 0,
                ull_avail_page_file: 0,
                ull_total_virtual: 0,
                ull_avail_virtual: 0,
                ull_avail_extended_virtual: 0,
            };
            if GlobalMemoryStatusEx(&mut status) != 0 && status.ull_total_phys > 0 {
                return SystemMemoryInfo {
                    total_memory_mb: (status.ull_total_phys / (1024 * 1024)).max(2048),
                    available_memory_mb: (status.ull_avail_phys / (1024 * 1024)).max(512),
                };
            }
        }
    }

    // Safe fallback if OS query fails
    SystemMemoryInfo {
        total_memory_mb: 16384,
        available_memory_mb: 8192,
    }
}

impl UserSettings {
    pub fn sanitize(&mut self, sys_mem: &SystemMemoryInfo) {
        let max_allowed = ((sys_mem.total_memory_mb * 90) / 100)
            .clamp(MIN_MEMORY_MB as u64, 65536) as u32;

        self.java.allocated_memory_mb = self
            .java
            .allocated_memory_mb
            .clamp(MIN_MEMORY_MB, max_allowed);

        if self.game.resolution_width < 320 {
            self.game.resolution_width = DEFAULT_RESOLUTION_WIDTH;
        }
        if self.game.resolution_height < 240 {
            self.game.resolution_height = DEFAULT_RESOLUTION_HEIGHT;
        }

        let theme_norm = self.launcher.theme.trim().to_ascii_lowercase();
        if theme_norm != "latte" && theme_norm != "dark" {
            self.launcher.theme = "latte".to_string();
        } else {
            self.launcher.theme = theme_norm;
        }
    }

    fn init_and_save_defaults(paths: &SzzPaths, sys_mem: &SystemMemoryInfo) -> Self {
        let mut default_settings = Self::default();
        default_settings.sanitize(sys_mem);
        let _ = default_settings.write_sanitized_to_disk(paths);
        default_settings
    }

    fn write_sanitized_to_disk(&self, paths: &SzzPaths) -> Result<()> {
        let file_path = paths.settings_file();
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| SzzError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let json_str = serde_json::to_string_pretty(self)?;
        let pid = std::process::id();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let tmp_path = file_path.with_extension(format!("json.tmp.{}.{}", pid, nanos));

        std::fs::write(&tmp_path, json_str.as_bytes()).map_err(|source| SzzError::Io {
            path: tmp_path.clone(),
            source,
        })?;

        std::fs::rename(&tmp_path, &file_path).map_err(|source| {
            let _ = std::fs::remove_file(&tmp_path);
            SzzError::Io {
                path: file_path,
                source,
            }
        })
    }

    pub fn load(paths: &SzzPaths) -> Result<Self> {
        let file_path = paths.settings_file();
        let sys_mem = get_system_memory_info();

        match std::fs::read_to_string(&file_path) {
            Ok(raw) => match serde_json::from_str::<UserSettings>(&raw) {
                Ok(mut parsed) => {
                    parsed.sanitize(&sys_mem);
                    Ok(parsed)
                }
                Err(err) => {
                    eprintln!(
                        "[szz-core] Warning: settings.json is malformed ({}), restoring defaults",
                        err
                    );
                    Ok(Self::init_and_save_defaults(paths, &sys_mem))
                }
            },
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                Ok(Self::init_and_save_defaults(paths, &sys_mem))
            }
            Err(source) => Err(SzzError::Io {
                path: file_path,
                source,
            }),
        }
    }

    pub fn save(&self, paths: &SzzPaths) -> Result<Self> {
        let mut sanitized = self.clone();
        sanitized.sanitize(&get_system_memory_info());
        sanitized.write_sanitized_to_disk(paths)?;
        Ok(sanitized)
    }

    pub fn reset(paths: &SzzPaths) -> Result<Self> {
        Self::default().save(paths)
    }

    pub fn get_by_key(&self, key: &str) -> Result<String> {
        match key.trim() {
            "game.resolution_width" | "game.resolutionWidth" => {
                Ok(self.game.resolution_width.to_string())
            }
            "game.resolution_height" | "game.resolutionHeight" => {
                Ok(self.game.resolution_height.to_string())
            }
            "game.fullscreen" => Ok(self.game.fullscreen.to_string()),
            "game.auto_connect" | "game.autoConnect" => Ok(self.game.auto_connect.to_string()),
            "java.allocated_memory_mb" | "java.allocatedMemoryMb" => {
                Ok(self.java.allocated_memory_mb.to_string())
            }
            "java.custom_jvm_args" | "java.customJvmArgs" => {
                Ok(self.java.custom_jvm_args.join(" "))
            }
            "launcher.behavior_on_launch" | "launcher.behaviorOnLaunch" => {
                Ok(self.launcher.behavior_on_launch.as_str().to_string())
            }
            "launcher.theme" => Ok(self.launcher.theme.clone()),
            other => Err(SzzError::Config(format!("Unknown settings key: '{}'", other))),
        }
    }

    pub fn set_by_key(&mut self, key: &str, value: &str) -> Result<()> {
        let val = value.trim();
        match key.trim() {
            "game.resolution_width" | "game.resolutionWidth" => {
                let parsed = val.parse::<u32>().map_err(|_| {
                    SzzError::Config(format!("Invalid integer for resolution_width: '{}'", val))
                })?;
                if parsed < 320 {
                    return Err(SzzError::Config(
                        "resolution_width must be at least 320".to_string(),
                    ));
                }
                self.game.resolution_width = parsed;
            }
            "game.resolution_height" | "game.resolutionHeight" => {
                let parsed = val.parse::<u32>().map_err(|_| {
                    SzzError::Config(format!("Invalid integer for resolution_height: '{}'", val))
                })?;
                if parsed < 240 {
                    return Err(SzzError::Config(
                        "resolution_height must be at least 240".to_string(),
                    ));
                }
                self.game.resolution_height = parsed;
            }
            "game.fullscreen" => {
                let parsed = val.parse::<bool>().map_err(|_| {
                    SzzError::Config(format!("Invalid boolean for fullscreen: '{}'", val))
                })?;
                self.game.fullscreen = parsed;
            }
            "game.auto_connect" | "game.autoConnect" => {
                let parsed = val.parse::<bool>().map_err(|_| {
                    SzzError::Config(format!("Invalid boolean for auto_connect: '{}'", val))
                })?;
                self.game.auto_connect = parsed;
            }
            "java.allocated_memory_mb" | "java.allocatedMemoryMb" => {
                let parsed = val.parse::<u32>().map_err(|_| {
                    SzzError::Config(format!("Invalid integer for allocated_memory_mb: '{}'", val))
                })?;
                if parsed < MIN_MEMORY_MB {
                    return Err(SzzError::Config(format!(
                        "allocated_memory_mb must be at least {} MB",
                        MIN_MEMORY_MB
                    )));
                }
                self.java.allocated_memory_mb = parsed;
            }
            "java.custom_jvm_args" | "java.customJvmArgs" => {
                self.java.custom_jvm_args = val
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();
            }
            "launcher.behavior_on_launch" | "launcher.behaviorOnLaunch" => {
                let parsed = LauncherBehaviorOnLaunch::from_str_value(val).ok_or_else(|| {
                    SzzError::Config(format!(
                        "Invalid behavior_on_launch '{}' (expected: keep_open, minimize, close)",
                        val
                    ))
                })?;
                self.launcher.behavior_on_launch = parsed;
            }
            "launcher.theme" => {
                let lower = val.to_ascii_lowercase();
                if lower != "latte" && lower != "dark" {
                    return Err(SzzError::Config(format!(
                        "Invalid theme '{}' (expected: latte, dark)",
                        val
                    )));
                }
                self.launcher.theme = lower;
            }
            other => {
                return Err(SzzError::Config(format!("Unknown settings key: '{}'", other)));
            }
        }

        self.sanitize(&get_system_memory_info());
        Ok(())
    }
}
