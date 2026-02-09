//! Application scanner - finds installed apps and their sizes.

use std::fs;
use std::path::{Path, PathBuf};

/// Information about an installed application.
#[derive(Debug, Clone)]
pub struct InstalledApp {
    /// Name of the application.
    pub name: String,
    /// Path to the .app bundle.
    pub path: PathBuf,
    /// Size of the .app bundle in bytes.
    pub size: u64,
    /// Bundle identifier (e.g., com.apple.Safari).
    pub bundle_id: Option<String>,
    /// Paths to leftover files in ~/Library.
    pub leftovers: Vec<PathBuf>,
    /// Total size of leftovers.
    pub leftover_size: u64,
}

/// Scan for installed applications.
pub fn scan_applications() -> Vec<InstalledApp> {
    let mut apps = Vec::new();

    // Scan /Applications
    if let Ok(entries) = fs::read_dir("/Applications") {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().map(|e| e == "app").unwrap_or(false) {
                if let Some(app) = scan_app_bundle(&path) {
                    apps.push(app);
                }
            }
        }
    }

    // Scan ~/Applications
    if let Some(home) = dirs::home_dir() {
        let user_apps = home.join("Applications");
        if let Ok(entries) = fs::read_dir(&user_apps) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().map(|e| e == "app").unwrap_or(false) {
                    if let Some(app) = scan_app_bundle(&path) {
                        apps.push(app);
                    }
                }
            }
        }
    }

    // Sort by size (largest first)
    apps.sort_by(|a, b| b.size.cmp(&a.size));

    apps
}

/// Scan a single .app bundle.
fn scan_app_bundle(path: &Path) -> Option<InstalledApp> {
    let name = path.file_stem()?.to_string_lossy().to_string();
    let size = calculate_dir_size(path);

    // Try to get bundle ID from Info.plist
    let bundle_id = get_bundle_id(path);

    // Find leftovers
    let (leftovers, leftover_size) = find_app_leftovers(&name, bundle_id.as_deref());

    Some(InstalledApp {
        name,
        path: path.to_path_buf(),
        size,
        bundle_id,
        leftovers,
        leftover_size,
    })
}

/// Get bundle identifier from Info.plist.
fn get_bundle_id(app_path: &Path) -> Option<String> {
    let plist_path = app_path.join("Contents/Info.plist");
    if !plist_path.exists() {
        return None;
    }

    // Simple plist parsing - look for CFBundleIdentifier
    let content = fs::read_to_string(&plist_path).ok()?;

    // Find CFBundleIdentifier key and extract value
    if let Some(pos) = content.find("<key>CFBundleIdentifier</key>") {
        let after_key = &content[pos..];
        if let Some(start) = after_key.find("<string>") {
            let value_start = start + 8;
            if let Some(end) = after_key[value_start..].find("</string>") {
                return Some(after_key[value_start..value_start + end].to_string());
            }
        }
    }

    None
}

/// Find leftover files for an app in ~/Library.
fn find_app_leftovers(app_name: &str, bundle_id: Option<&str>) -> (Vec<PathBuf>, u64) {
    let mut leftovers = Vec::new();
    let mut total_size = 0u64;

    let Some(home) = dirs::home_dir() else {
        return (leftovers, total_size);
    };

    let library = home.join("Library");

    // Directories to search for leftovers
    let search_dirs = [
        "Application Support",
        "Caches",
        "Preferences",
        "Logs",
        "Containers",
        "Saved Application State",
    ];

    // Patterns to match (app name or bundle ID)
    let mut patterns: Vec<String> = vec![app_name.to_lowercase()];
    if let Some(bid) = bundle_id {
        patterns.push(bid.to_lowercase());
        // Also try parts of bundle ID (e.g., "com.company.app" -> "company.app", "app")
        for part in bid.split('.').skip(1) {
            if part.len() > 3 {
                patterns.push(part.to_lowercase());
            }
        }
    }

    for dir_name in &search_dirs {
        let search_path = library.join(dir_name);
        if !search_path.exists() {
            continue;
        }

        if let Ok(entries) = fs::read_dir(&search_path) {
            for entry in entries.filter_map(|e| e.ok()) {
                let entry_name = entry.file_name().to_string_lossy().to_lowercase();

                // Check if entry matches any pattern
                for pattern in &patterns {
                    if entry_name.contains(pattern) {
                        let path = entry.path();
                        let size = if path.is_dir() {
                            calculate_dir_size(&path)
                        } else {
                            path.metadata().map(|m| m.len()).unwrap_or(0)
                        };

                        leftovers.push(path);
                        total_size += size;
                        break;
                    }
                }
            }
        }
    }

    (leftovers, total_size)
}

/// Calculate directory size recursively.
fn calculate_dir_size(path: &Path) -> u64 {
    walkdir::WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_applications() {
        let apps = scan_applications();
        // Should find at least some apps on macOS
        assert!(!apps.is_empty() || cfg!(not(target_os = "macos")));
    }
}
