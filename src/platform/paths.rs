//! Platform-specific path definitions.
//!
//! This module provides cross-platform path mappings for:
//! - User cache directories
//! - User log directories
//! - Application support directories
//! - System application directories

use std::path::PathBuf;

/// Platform-specific standard paths.
pub struct PlatformPaths;

impl PlatformPaths {
    /// Get user cache directories.
    #[cfg(target_os = "macos")]
    pub fn cache_dirs() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("Library/Caches"));
        }
        paths
    }

    /// Get user cache directories.
    #[cfg(target_os = "linux")]
    pub fn cache_dirs() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(cache) = dirs::cache_dir() {
            paths.push(cache);
        }
        // Also check ~/.cache directly as fallback
        if let Some(home) = dirs::home_dir() {
            let cache = home.join(".cache");
            if cache.exists() && !paths.contains(&cache) {
                paths.push(cache);
            }
        }
        paths
    }

    /// Get user cache directories.
    #[cfg(target_os = "windows")]
    pub fn cache_dirs() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(local) = dirs::cache_dir() {
            paths.push(local); // %LOCALAPPDATA%
        }
        // Windows Temp
        if let Some(home) = dirs::home_dir() {
            let temp = home.join("AppData\\Local\\Temp");
            if temp.exists() {
                paths.push(temp);
            }
        }
        paths
    }

    /// Get user log directories.
    #[cfg(target_os = "macos")]
    pub fn log_dirs() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("Library/Logs"));
        }
        paths
    }

    /// Get user log directories.
    #[cfg(target_os = "linux")]
    pub fn log_dirs() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            let logs = home.join(".local/share/logs");
            if logs.exists() {
                paths.push(logs);
            }
            // XDG state directory for logs
            let state = home.join(".local/state");
            if state.exists() {
                paths.push(state);
            }
        }
        paths
    }

    /// Get user log directories.
    #[cfg(target_os = "windows")]
    pub fn log_dirs() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(local) = dirs::data_local_dir() {
            let logs = local.join("Logs");
            if logs.exists() {
                paths.push(logs);
            }
        }
        paths
    }

    /// Get application support directories (for leftovers detection).
    #[cfg(target_os = "macos")]
    pub fn app_support_dirs() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("Library/Application Support"));
            paths.push(home.join("Library/Preferences"));
            paths.push(home.join("Library/Caches"));
            paths.push(home.join("Library/Containers"));
            paths.push(home.join("Library/Group Containers"));
            paths.push(home.join("Library/Saved Application State"));
        }
        paths
    }

    /// Get application support directories.
    #[cfg(target_os = "linux")]
    pub fn app_support_dirs() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(data) = dirs::data_dir() {
            paths.push(data);
        }
        if let Some(config) = dirs::config_dir() {
            paths.push(config);
        }
        if let Some(home) = dirs::home_dir() {
            let local_share = home.join(".local/share");
            if local_share.exists() && !paths.contains(&local_share) {
                paths.push(local_share);
            }
            let config = home.join(".config");
            if config.exists() && !paths.contains(&config) {
                paths.push(config);
            }
        }
        paths
    }

    /// Get application support directories.
    #[cfg(target_os = "windows")]
    pub fn app_support_dirs() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(local) = dirs::data_local_dir() {
            paths.push(local.clone());
        }
        if let Some(roaming) = dirs::data_dir() {
            paths.push(roaming.clone());
        }
        if let Some(home) = dirs::home_dir() {
            let local = home.join("AppData\\Local");
            if local.exists() && !paths.contains(&local) {
                paths.push(local);
            }
            let roaming = home.join("AppData\\Roaming");
            if roaming.exists() && !paths.contains(&roaming) {
                paths.push(roaming);
            }
        }
        paths
    }

    /// Get system application directories.
    #[cfg(target_os = "macos")]
    pub fn application_dirs() -> Vec<PathBuf> {
        let mut paths = vec![PathBuf::from("/Applications")];
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("Applications"));
        }
        paths
    }

    /// Get system application directories.
    #[cfg(target_os = "linux")]
    pub fn application_dirs() -> Vec<PathBuf> {
        let mut paths = vec![
            PathBuf::from("/usr/share/applications"),
            PathBuf::from("/usr/local/share/applications"),
        ];
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".local/share/applications"));
        }
        paths
    }

    /// Get system application directories.
    #[cfg(target_os = "windows")]
    pub fn application_dirs() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Ok(program_files) = std::env::var("ProgramFiles") {
            paths.push(PathBuf::from(program_files));
        }
        if let Ok(program_files_x86) = std::env::var("ProgramFiles(x86)") {
            paths.push(PathBuf::from(program_files_x86));
        }
        if let Some(local) = dirs::data_local_dir() {
            let programs = local.join("Programs");
            if programs.exists() {
                paths.push(programs);
            }
        }
        paths
    }

    /// Get the user's trash directory path.
    #[cfg(target_os = "macos")]
    pub fn trash_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".Trash"))
    }

    /// Get the user's trash directory path.
    #[cfg(target_os = "linux")]
    pub fn trash_dir() -> Option<PathBuf> {
        // XDG Trash specification
        dirs::data_local_dir()
            .or_else(dirs::home_dir)
            .map(|p| p.join(".local/share/Trash/files"))
    }

    /// Get the user's trash directory path.
    /// Note: On Windows, the Recycle Bin is managed by the system.
    #[cfg(target_os = "windows")]
    pub fn trash_dir() -> Option<PathBuf> {
        // Windows Recycle Bin is per-drive and managed by the system
        // Return None as we use the trash crate for proper handling
        None
    }

    /// Get the current operating system name.
    pub fn current_os() -> &'static str {
        #[cfg(target_os = "macos")]
        {
            "macos"
        }
        #[cfg(target_os = "linux")]
        {
            "linux"
        }
        #[cfg(target_os = "windows")]
        {
            "windows"
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            "unknown"
        }
    }

    /// Check if the current platform is Unix-like.
    pub fn is_unix() -> bool {
        cfg!(unix)
    }

    /// Check if the current platform is Windows.
    pub fn is_windows() -> bool {
        cfg!(windows)
    }

    /// Get npm cache directory.
    pub fn npm_cache_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".npm"))
    }

    /// Get yarn cache directories.
    #[cfg(target_os = "macos")]
    pub fn yarn_cache_dirs() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".yarn/cache"));
            paths.push(home.join("Library/Caches/Yarn"));
        }
        paths
    }

    /// Get yarn cache directories.
    #[cfg(target_os = "linux")]
    pub fn yarn_cache_dirs() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".yarn/cache"));
        }
        if let Some(cache) = dirs::cache_dir() {
            paths.push(cache.join("yarn"));
        }
        paths
    }

    /// Get yarn cache directories.
    #[cfg(target_os = "windows")]
    pub fn yarn_cache_dirs() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".yarn\\cache"));
        }
        if let Some(local) = dirs::data_local_dir() {
            paths.push(local.join("Yarn\\Cache"));
        }
        paths
    }

    /// Get pip cache directory.
    #[cfg(target_os = "macos")]
    pub fn pip_cache_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join("Library/Caches/pip"))
    }

    /// Get pip cache directory.
    #[cfg(target_os = "linux")]
    pub fn pip_cache_dir() -> Option<PathBuf> {
        dirs::cache_dir()
            .map(|c| c.join("pip"))
            .or_else(|| dirs::home_dir().map(|h| h.join(".cache/pip")))
    }

    /// Get pip cache directory.
    #[cfg(target_os = "windows")]
    pub fn pip_cache_dir() -> Option<PathBuf> {
        dirs::data_local_dir().map(|l| l.join("pip\\Cache"))
    }

    /// Get cargo cache directories.
    pub fn cargo_cache_dirs() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".cargo/registry/cache"));
            paths.push(home.join(".cargo/registry/src"));
        }
        paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_dirs_not_empty() {
        let dirs = PlatformPaths::cache_dirs();
        // At least one cache dir should be returned
        assert!(!dirs.is_empty() || dirs::home_dir().is_none());
    }

    #[test]
    fn test_current_os() {
        let os = PlatformPaths::current_os();
        assert!(["macos", "linux", "windows", "unknown"].contains(&os));
    }

    #[test]
    fn test_npm_cache_dir() {
        if dirs::home_dir().is_some() {
            assert!(PlatformPaths::npm_cache_dir().is_some());
        }
    }

    #[test]
    fn test_cargo_cache_dirs() {
        if dirs::home_dir().is_some() {
            let dirs = PlatformPaths::cargo_cache_dirs();
            assert!(!dirs.is_empty());
        }
    }
}
