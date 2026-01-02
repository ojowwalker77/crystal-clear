//! Configuration file loading and parsing.

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::schema::Config;
use crate::error::CleanmacError;

/// Default configuration file path.
pub fn default_config_path() -> PathBuf {
    dirs::config_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
        .unwrap_or_else(|| PathBuf::from("/tmp/.config"))
        .join("crystal-clear")
        .join("config.toml")
}

/// Load configuration from the default location or a custom path.
pub fn load_config(custom_path: Option<&Path>) -> Result<Config, CleanmacError> {
    let path = custom_path
        .map(|p| p.to_path_buf())
        .unwrap_or_else(default_config_path);

    if !path.exists() {
        // Return default config if file doesn't exist
        return Ok(Config::default());
    }

    let content = fs::read_to_string(&path).map_err(|e| CleanmacError::Config {
        message: format!("Failed to read config file '{}': {}", path.display(), e),
    })?;

    parse_config(&content)
}

/// Parse configuration from TOML string.
pub fn parse_config(content: &str) -> Result<Config, CleanmacError> {
    toml::from_str(content).map_err(|e| CleanmacError::Config {
        message: format!("Failed to parse config: {}", e),
    })
}

/// Create a default configuration file.
pub fn create_default_config(path: &Path) -> Result<(), CleanmacError> {
    let _config = Config::default();
    let content = generate_default_config_toml();

    // Create parent directories if needed
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| CleanmacError::Config {
            message: format!("Failed to create config directory: {}", e),
        })?;
    }

    fs::write(path, content).map_err(|e| CleanmacError::Config {
        message: format!("Failed to write config file: {}", e),
    })?;

    Ok(())
}

/// Generate the default configuration file content with comments.
pub fn generate_default_config_toml() -> String {
    r#"# Crystal Clear Configuration File
# Location: ~/.config/crystal-clear/config.toml

[general]
# Default to dry-run mode (shows what would be cleaned without actually cleaning)
dry_run_default = true

# Keep files newer than this many days (0 = clean all regardless of age)
keep_recent_days = 0

# Output format: "table", "json", or "plain"
output_format = "table"

# Enable colored output
color = true

# Verbosity level (0 = quiet, 1 = normal, 2 = verbose, 3 = debug)
verbosity = 1

[safety]
# Show warning when cleaning more than this many bytes (10GB)
warn_threshold = 10737418240

# Require --force flag when cleaning more than this many bytes (50GB)
force_threshold = 53687091200

# Always ask for confirmation before cleaning
always_confirm = true

# Enable audit logging (records all cleaning operations)
audit_enabled = true

# Keep audit logs for this many days
audit_retention_days = 90

# Additional paths to protect (on top of hardcoded system paths)
# protected_paths = [
#     "~/Projects",
#     "~/Work",
# ]

# Global exclusions (these paths will never be cleaned)
# exclusions = [
#     "~/Library/Caches/important-app",
# ]

[categories.system_cache]
enabled = true
# exclude = ["~/Library/Caches/com.apple.Safari"]

[categories.system_logs]
enabled = true

[categories.trash]
enabled = true
# Only clean items older than this many days
min_age_days = 0

[categories.xcode]
enabled = true
# Keep archives newer than this many days
keep_recent_archives_days = 0
# iOS versions to always keep (e.g., ["17.0", "17.1"])
keep_ios_versions = []

[categories.homebrew]
enabled = true
# Number of old versions to keep per formula
keep_versions = 0

[categories.npm]
enabled = true

[categories.yarn]
enabled = true

[categories.cargo]
enabled = true
# Keep crates used in the last N days
keep_recent_days = 0

[categories.pip]
enabled = true

[categories.docker]
enabled = true
# Only clean dangling images (safer)
dangling_only = true
# Clean unused volumes (CAUTION: may contain important data!)
clean_volumes = false
# Clean build cache
clean_build_cache = true

[categories.apps]
enabled = true
# Known uninstalled app bundle IDs to look for leftovers
# known_apps = ["com.example.uninstalled-app"]
"#
    .to_string()
}

/// Validate the configuration.
pub fn validate_config(config: &Config) -> Result<(), CleanmacError> {
    // Validate protected paths exist (if specified)
    for path in &config.safety.protected_paths {
        let expanded = expand_tilde(path);
        if !expanded.exists() {
            // Warning only, not an error
            tracing::warn!(
                "Protected path does not exist: {}",
                expanded.display()
            );
        }
    }

    // Validate exclusions
    for path in &config.exclusions {
        let expanded = expand_tilde(path);
        if !expanded.exists() {
            tracing::warn!("Exclusion path does not exist: {}", expanded.display());
        }
    }

    Ok(())
}

/// Expand ~ to the home directory.
pub fn expand_tilde(path: &Path) -> PathBuf {
    if let Ok(stripped) = path.strip_prefix("~") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_parses() {
        let content = generate_default_config_toml();
        let config = parse_config(&content).unwrap();
        assert!(config.general.dry_run_default);
        assert!(config.safety.always_confirm);
    }

    #[test]
    fn test_empty_config_uses_defaults() {
        let config = parse_config("").unwrap();
        assert!(config.general.dry_run_default);
        assert_eq!(config.safety.warn_threshold, 10 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_partial_config_merged_with_defaults() {
        let content = r#"
[general]
dry_run_default = false
"#;
        let config = parse_config(content).unwrap();
        assert!(!config.general.dry_run_default);
        // Other fields should use defaults
        assert!(config.safety.always_confirm);
    }

    #[test]
    fn test_expand_tilde() {
        let path = PathBuf::from("~/test");
        let expanded = expand_tilde(&path);
        assert!(!expanded.starts_with("~"));
        if let Some(home) = dirs::home_dir() {
            assert!(expanded.starts_with(home));
        }
    }
}
