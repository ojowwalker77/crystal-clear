//! Hardcoded protected paths that are NEVER touched.
//!
//! CRITICAL: This module defines the absolute safety boundaries of Crystal Clear.
//! These paths cannot be overridden by any configuration or command-line flag.
//! Modifying this list requires extreme caution and thorough review.
//!
//! Platform-specific protection:
//! - macOS: System directories, Library paths, Keychains
//! - Linux: System directories, XDG paths
//! - Windows: System directories, Program Files, AppData security paths

use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::path::PathBuf;

/// Absolute system paths that are NEVER touched under any circumstances.
/// This list is hardcoded and cannot be modified at runtime.
pub static PROTECTED_SYSTEM_PATHS: Lazy<HashSet<PathBuf>> = Lazy::new(|| {
    let mut paths = HashSet::new();

    // Root filesystem - NEVER touch on Unix systems
    #[cfg(not(target_os = "windows"))]
    paths.insert(PathBuf::from("/"));

    // macOS-specific system directories
    #[cfg(target_os = "macos")]
    {
        paths.insert(PathBuf::from("/System"));
        paths.insert(PathBuf::from("/Library"));
        paths.insert(PathBuf::from("/Applications"));
        paths.insert(PathBuf::from("/Users"));
        paths.insert(PathBuf::from("/bin"));
        paths.insert(PathBuf::from("/sbin"));
        paths.insert(PathBuf::from("/usr"));
        paths.insert(PathBuf::from("/var"));
        paths.insert(PathBuf::from("/etc"));
        paths.insert(PathBuf::from("/tmp"));
        paths.insert(PathBuf::from("/private"));
        paths.insert(PathBuf::from("/cores"));
        paths.insert(PathBuf::from("/opt"));
        paths.insert(PathBuf::from("/Volumes"));
        paths.insert(PathBuf::from("/dev"));
        paths.insert(PathBuf::from("/Network"));
        paths.insert(PathBuf::from("/net"));
    }

    // Linux-specific system directories
    #[cfg(target_os = "linux")]
    {
        paths.insert(PathBuf::from("/bin"));
        paths.insert(PathBuf::from("/sbin"));
        paths.insert(PathBuf::from("/usr"));
        paths.insert(PathBuf::from("/var"));
        paths.insert(PathBuf::from("/etc"));
        paths.insert(PathBuf::from("/tmp"));
        paths.insert(PathBuf::from("/opt"));
        paths.insert(PathBuf::from("/dev"));
        paths.insert(PathBuf::from("/proc"));
        paths.insert(PathBuf::from("/sys"));
        paths.insert(PathBuf::from("/boot"));
        paths.insert(PathBuf::from("/lib"));
        paths.insert(PathBuf::from("/lib64"));
        paths.insert(PathBuf::from("/root"));
        paths.insert(PathBuf::from("/home"));
        paths.insert(PathBuf::from("/mnt"));
        paths.insert(PathBuf::from("/media"));
        paths.insert(PathBuf::from("/run"));
        paths.insert(PathBuf::from("/srv"));
    }

    // Windows-specific system directories
    #[cfg(target_os = "windows")]
    {
        // Common Windows paths (always protected)
        paths.insert(PathBuf::from("C:\\Windows"));
        paths.insert(PathBuf::from("C:\\Windows\\System32"));
        paths.insert(PathBuf::from("C:\\Windows\\SysWOW64"));
        paths.insert(PathBuf::from("C:\\Program Files"));
        paths.insert(PathBuf::from("C:\\Program Files (x86)"));
        paths.insert(PathBuf::from("C:\\Users"));
        paths.insert(PathBuf::from("C:\\ProgramData"));

        // Environment-based paths (may differ per system)
        if let Ok(windir) = std::env::var("WINDIR") {
            paths.insert(PathBuf::from(&windir));
            paths.insert(PathBuf::from(format!("{}\\System32", windir)));
            paths.insert(PathBuf::from(format!("{}\\SysWOW64", windir)));
        }
        if let Ok(program_files) = std::env::var("ProgramFiles") {
            paths.insert(PathBuf::from(program_files));
        }
        if let Ok(program_files_x86) = std::env::var("ProgramFiles(x86)") {
            paths.insert(PathBuf::from(program_files_x86));
        }
        if let Ok(program_data) = std::env::var("ProgramData") {
            paths.insert(PathBuf::from(program_data));
        }
    }

    paths
});

/// User-specific paths that are NEVER touched.
/// These are relative to the user's home directory.
pub static PROTECTED_USER_PATHS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    let mut paths = vec![
        // Core user directories (cross-platform)
        "Desktop",
        "Documents",
        "Downloads",
        // Security-critical directories (cross-platform)
        ".ssh",
        ".gnupg",
        ".aws",
        ".kube",
        ".docker/certs.d",
    ];

    // macOS-specific protected paths
    #[cfg(target_os = "macos")]
    {
        paths.extend([
            "Movies",
            "Music",
            "Pictures",
            "Public",
            "Library/Keychains",
            "Library/Mail",
            "Library/Messages",
            "Library/Safari",
            "Library/Calendars",
            "Library/Contacts",
            "Library/Notes",
            "Library/Reminders",
            "Library/HomeKit",
            "Library/Accounts",
            "Library/IdentityServices",
            "Library/Passwords",
            "Library/Application Support/MobileSync",
            "Library/Preferences/com.apple.security.plist",
        ]);
    }

    // Linux-specific protected paths
    #[cfg(target_os = "linux")]
    {
        paths.extend([
            "Music",
            "Pictures",
            "Videos",
            "Public",
            ".local/share/keyrings",
            ".mozilla",
            ".thunderbird",
            ".config/chromium",
            ".config/google-chrome",
            ".pki",
        ]);
    }

    // Windows-specific protected paths
    #[cfg(target_os = "windows")]
    {
        paths.extend([
            "Music",
            "Pictures",
            "Videos",
            "AppData\\Roaming\\Microsoft\\Credentials",
            "AppData\\Roaming\\Microsoft\\Protect",
            "AppData\\Local\\Microsoft\\Credentials",
            "AppData\\Roaming\\Microsoft\\SystemCertificates",
        ]);
    }

    paths
});

/// File patterns that are never touched (glob patterns).
pub static PROTECTED_PATTERNS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    let mut patterns = vec![
        // Security files (cross-platform)
        "*.keychain",
        "*.keychain-db",
        "*.p12",
        "*.pfx",
        "*.cer",
        "*.pem",
        "*.key",
        // Git internals (dangerous to delete)
        ".git",
    ];

    // macOS-specific patterns
    #[cfg(target_os = "macos")]
    {
        patterns.extend([
            "*.mobileprovision",
            ".DS_Store",
            ".localized",
        ]);
    }

    // Windows-specific patterns
    #[cfg(target_os = "windows")]
    {
        patterns.extend([
            "desktop.ini",
            "thumbs.db",
            "Thumbs.db",
        ]);
    }

    patterns
});

/// Get all protected paths for the current user.
pub fn get_protected_paths() -> HashSet<PathBuf> {
    let mut paths = PROTECTED_SYSTEM_PATHS.clone();

    if let Some(home) = dirs::home_dir() {
        // Add the home directory itself
        paths.insert(home.clone());

        // Add all user-specific protected paths
        for relative_path in PROTECTED_USER_PATHS.iter() {
            paths.insert(home.join(relative_path));
        }
    }

    paths
}

/// Check if a path matches any protected pattern.
pub fn matches_protected_pattern(path: &std::path::Path) -> bool {
    let file_name = match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => return false,
    };

    for pattern in PROTECTED_PATTERNS.iter() {
        if pattern.starts_with('*') {
            // Suffix match (e.g., "*.keychain")
            let suffix = &pattern[1..];
            if file_name.ends_with(suffix) {
                return true;
            }
        } else {
            // Exact match on filename (case-insensitive on Windows)
            #[cfg(target_os = "windows")]
            {
                if file_name.eq_ignore_ascii_case(pattern) {
                    return true;
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                if file_name == *pattern {
                    return true;
                }
            }
        }
    }

    // Only check ancestors for .git directories (to protect git internals)
    for ancestor in path.ancestors().skip(1) {
        if let Some(name) = ancestor.file_name().and_then(|n| n.to_str()) {
            if name == ".git" {
                return true;
            }
        }
    }

    false
}

/// Check if a path is protected (exact match or is under a protected path).
pub fn is_protected(path: &std::path::Path) -> bool {
    // Get both original and canonical paths for checking
    let original_path = path.to_path_buf();
    let canonical_path = path.canonicalize().unwrap_or_else(|_| original_path.clone());

    // Check file patterns first (like .keychain, .git, etc)
    if matches_protected_pattern(&original_path) || matches_protected_pattern(&canonical_path) {
        return true;
    }

    // Paths that are protected but we DON'T protect their children
    // (we just don't want to delete the directory itself)
    let mut exact_only: HashSet<PathBuf> = HashSet::new();

    #[cfg(not(target_os = "windows"))]
    {
        exact_only.insert(PathBuf::from("/"));
    }

    #[cfg(target_os = "macos")]
    {
        exact_only.insert(PathBuf::from("/Users"));
    }

    #[cfg(target_os = "linux")]
    {
        exact_only.insert(PathBuf::from("/home"));
    }

    #[cfg(target_os = "windows")]
    {
        exact_only.insert(PathBuf::from("C:\\Users"));
    }

    if let Some(home) = dirs::home_dir() {
        exact_only.insert(home);
    }

    // Check exact match for exact-only paths (check both original and canonical)
    if exact_only.contains(&original_path) || exact_only.contains(&canonical_path) {
        return true;
    }

    // User paths where we protect EVERYTHING underneath
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return false,
    };

    let protected_trees: Vec<PathBuf> = PROTECTED_USER_PATHS
        .iter()
        .map(|p| home.join(p))
        .collect();

    // Check if path is under a protected user tree (Documents, Desktop, .ssh, etc)
    for protected_path in &protected_trees {
        if canonical_path.starts_with(protected_path) {
            return true;
        }
    }

    // System paths - exact match only (check both original and canonical)
    // We protect the directories themselves, not their children
    if PROTECTED_SYSTEM_PATHS.contains(&original_path) || PROTECTED_SYSTEM_PATHS.contains(&canonical_path) {
        return true;
    }

    false
}

/// Get a human-readable reason why a path is protected.
pub fn get_protection_reason(path: &std::path::Path) -> Option<String> {
    if is_protected(path) {
        Some(format!("'{}' is protected", path.display()))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_paths_protected() {
        #[cfg(target_os = "macos")]
        {
            assert!(is_protected(&PathBuf::from("/")));
            assert!(is_protected(&PathBuf::from("/System")));
            assert!(is_protected(&PathBuf::from("/Applications")));
            assert!(is_protected(&PathBuf::from("/Library")));
            assert!(is_protected(&PathBuf::from("/bin")));
            assert!(is_protected(&PathBuf::from("/usr")));
        }

        #[cfg(target_os = "linux")]
        {
            assert!(is_protected(&PathBuf::from("/")));
            assert!(is_protected(&PathBuf::from("/usr")));
            assert!(is_protected(&PathBuf::from("/etc")));
            // Note: /bin and /proc may not exist or be symlinks on some distros
        }

        #[cfg(target_os = "windows")]
        {
            assert!(is_protected(&PathBuf::from("C:\\Windows")));
            assert!(is_protected(&PathBuf::from("C:\\Program Files")));
        }
    }

    #[test]
    fn test_user_paths_protected() {
        if let Some(home) = dirs::home_dir() {
            assert!(is_protected(&home.join("Documents")));
            assert!(is_protected(&home.join("Desktop")));
            assert!(is_protected(&home.join(".ssh")));
            assert!(is_protected(&home.join(".gnupg")));

            #[cfg(target_os = "macos")]
            assert!(is_protected(&home.join("Library/Keychains")));

            #[cfg(target_os = "linux")]
            assert!(is_protected(&home.join(".local/share/keyrings")));
        }
    }

    #[test]
    fn test_patterns_protected() {
        assert!(matches_protected_pattern(&PathBuf::from(
            "/some/path/login.keychain"
        )));
        assert!(matches_protected_pattern(&PathBuf::from(
            "/some/path/cert.p12"
        )));

        #[cfg(target_os = "macos")]
        assert!(matches_protected_pattern(&PathBuf::from(
            "/some/path/.DS_Store"
        )));

        #[cfg(target_os = "windows")]
        assert!(matches_protected_pattern(&PathBuf::from(
            "C:\\some\\path\\desktop.ini"
        )));
    }

    #[test]
    fn test_safe_paths_not_protected() {
        if let Some(home) = dirs::home_dir() {
            #[cfg(target_os = "macos")]
            {
                let caches = home.join("Library/Caches");
                let logs = home.join("Library/Logs");
                assert!(!PROTECTED_SYSTEM_PATHS.contains(&caches));
                assert!(!PROTECTED_SYSTEM_PATHS.contains(&logs));
            }

            #[cfg(target_os = "linux")]
            {
                let cache = home.join(".cache");
                assert!(!PROTECTED_SYSTEM_PATHS.contains(&cache));
            }
        }
    }
}
