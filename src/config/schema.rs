//! Configuration schema definitions.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// General settings.
    pub general: GeneralConfig,
    /// Safety settings.
    pub safety: SafetyConfig,
    /// TUI-specific settings.
    pub tui: TuiConfig,
    /// Category-specific settings.
    pub categories: CategoryConfigs,
    /// Paths to always exclude from cleaning.
    pub exclusions: Vec<PathBuf>,
}

/// General application settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// Default to dry-run mode (recommended: true).
    pub dry_run_default: bool,
    /// Number of days to keep recent files (0 = clean all).
    pub keep_recent_days: u32,
    /// Output format: "table", "json", "plain".
    pub output_format: String,
    /// Enable colored output.
    pub color: bool,
    /// Verbosity level (0-3).
    pub verbosity: u8,
}

/// Safety-related settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SafetyConfig {
    /// Size threshold for warning (bytes). Default: 10GB.
    pub warn_threshold: u64,
    /// Size threshold requiring --force (bytes). Default: 50GB.
    pub force_threshold: u64,
    /// Always require confirmation before cleaning.
    pub always_confirm: bool,
    /// Additional protected paths (on top of hardcoded ones).
    pub protected_paths: Vec<PathBuf>,
    /// Enable audit logging.
    pub audit_enabled: bool,
    /// Audit log retention in days.
    pub audit_retention_days: u32,
}

/// TUI-specific settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TuiConfig {
    /// Hide zero-byte rows by default.
    pub hide_zero_byte_items: bool,
    /// Show clean report overlay when a clean operation completes.
    pub show_clean_report_on_complete: bool,
    /// Number of parallel scan workers (0 = auto).
    pub scan_parallelism: u8,
    /// Style used for confirm key hints.
    pub confirm_keys_hint_style: String,
}

/// Category-specific configurations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CategoryConfigs {
    pub system_cache: CategoryConfig,
    pub system_logs: CategoryConfig,
    pub trash: TrashConfig,
    pub xcode: XcodeConfig,
    pub homebrew: HomebrewConfig,
    pub npm: CategoryConfig,
    pub yarn: CategoryConfig,
    pub cargo: CargoConfig,
    pub pip: CategoryConfig,
    pub docker: DockerConfig,
    pub apps: AppsConfig,
}

/// Base category configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CategoryConfig {
    /// Enable this category.
    pub enabled: bool,
    /// Additional paths for this category.
    pub paths: Vec<PathBuf>,
    /// Paths to exclude within this category.
    pub exclude: Vec<PathBuf>,
    /// Patterns to exclude (glob).
    pub exclude_patterns: Vec<String>,
}

/// Trash-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TrashConfig {
    #[serde(flatten)]
    pub base: CategoryConfig,
    /// Minimum age in days before items can be cleaned from trash.
    pub min_age_days: u32,
}

/// Xcode-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct XcodeConfig {
    #[serde(flatten)]
    pub base: CategoryConfig,
    /// Keep archives newer than this many days.
    pub keep_recent_archives_days: u32,
    /// iOS versions to always keep.
    pub keep_ios_versions: Vec<String>,
}

/// Homebrew-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HomebrewConfig {
    #[serde(flatten)]
    pub base: CategoryConfig,
    /// Number of old versions to keep per formula.
    pub keep_versions: u32,
}

/// Cargo-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CargoConfig {
    #[serde(flatten)]
    pub base: CategoryConfig,
    /// Keep crates used in the last N days.
    pub keep_recent_days: u32,
}

/// Docker-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DockerConfig {
    #[serde(flatten)]
    pub base: CategoryConfig,
    /// Only clean dangling images (vs all unused).
    pub dangling_only: bool,
    /// Clean unused volumes (CAUTION: may contain data).
    pub clean_volumes: bool,
    /// Clean build cache.
    pub clean_build_cache: bool,
}

/// Apps leftovers configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppsConfig {
    #[serde(flatten)]
    pub base: CategoryConfig,
    /// Known app bundle identifiers to look for leftovers.
    pub known_apps: Vec<String>,
}

// ============================================================
// Default implementations
// ============================================================

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            safety: SafetyConfig::default(),
            tui: TuiConfig::default(),
            categories: CategoryConfigs::default(),
            exclusions: Vec::new(),
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            dry_run_default: true,
            keep_recent_days: 0,
            output_format: "table".to_string(),
            color: true,
            verbosity: 1,
        }
    }
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            warn_threshold: 10 * 1024 * 1024 * 1024,  // 10GB
            force_threshold: 50 * 1024 * 1024 * 1024, // 50GB
            always_confirm: true,
            protected_paths: Vec::new(),
            audit_enabled: true,
            audit_retention_days: 90,
        }
    }
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            hide_zero_byte_items: true,
            show_clean_report_on_complete: true,
            scan_parallelism: 0,
            confirm_keys_hint_style: "explicit".to_string(),
        }
    }
}

impl Default for CategoryConfigs {
    fn default() -> Self {
        Self {
            system_cache: CategoryConfig::default(),
            system_logs: CategoryConfig::default(),
            trash: TrashConfig::default(),
            xcode: XcodeConfig::default(),
            homebrew: HomebrewConfig::default(),
            npm: CategoryConfig::default(),
            yarn: CategoryConfig::default(),
            cargo: CargoConfig::default(),
            pip: CategoryConfig::default(),
            docker: DockerConfig::default(),
            apps: AppsConfig::default(),
        }
    }
}

impl Default for CategoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            paths: Vec::new(),
            exclude: Vec::new(),
            exclude_patterns: Vec::new(),
        }
    }
}

impl Default for TrashConfig {
    fn default() -> Self {
        Self {
            base: CategoryConfig::default(),
            min_age_days: 0,
        }
    }
}

impl Default for XcodeConfig {
    fn default() -> Self {
        Self {
            base: CategoryConfig::default(),
            keep_recent_archives_days: 0,
            keep_ios_versions: Vec::new(),
        }
    }
}

impl Default for HomebrewConfig {
    fn default() -> Self {
        Self {
            base: CategoryConfig::default(),
            keep_versions: 0,
        }
    }
}

impl Default for CargoConfig {
    fn default() -> Self {
        Self {
            base: CategoryConfig::default(),
            keep_recent_days: 0,
        }
    }
}

impl Default for DockerConfig {
    fn default() -> Self {
        Self {
            base: CategoryConfig::default(),
            dangling_only: true,
            clean_volumes: false, // Safe default
            clean_build_cache: true,
        }
    }
}

impl Default for AppsConfig {
    fn default() -> Self {
        Self {
            base: CategoryConfig::default(),
            known_apps: Vec::new(),
        }
    }
}
