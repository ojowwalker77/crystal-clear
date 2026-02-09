//! Heuristics for identifying cleanable directories.

use std::path::Path;

use crate::db::PathCategory;

/// Detect if a directory is cleanable based on heuristics.
/// Returns (category, confidence) if cleanable, None otherwise.
pub fn detect_cleanable_directory(path: &Path) -> Option<(PathCategory, f64)> {
    let name = path.file_name()?.to_str()?;
    let name_lower = name.to_lowercase();
    let path_str = path.to_string_lossy().to_lowercase();

    // High-confidence patterns (known cleanable locations)
    let high_confidence_patterns: &[(&str, PathCategory, f64)] = &[
        // Cache directories
        ("library/caches/", PathCategory::Cache, 0.95),
        ("_cacache", PathCategory::Cache, 0.95),
        (".cache/", PathCategory::Cache, 0.90),
        // Log directories
        ("library/logs/", PathCategory::Logs, 0.95),
        // Build artifacts
        ("deriveddata", PathCategory::Build, 0.95),
        (
            "library/developer/xcode/deriveddata",
            PathCategory::Build,
            0.98,
        ),
        // Package manager caches
        (".npm/_cacache", PathCategory::Cache, 0.95),
        (".cargo/registry/cache", PathCategory::Cache, 0.95),
        (".yarn/cache", PathCategory::Cache, 0.95),
    ];

    for (pattern, category, confidence) in high_confidence_patterns {
        if path_str.contains(pattern) {
            return Some((category.clone(), *confidence));
        }
    }

    // Name-based detection (medium confidence)
    let name_patterns: &[(&str, PathCategory, f64)] = &[
        ("caches", PathCategory::Cache, 0.75),
        ("cache", PathCategory::Cache, 0.7),
        ("logs", PathCategory::Logs, 0.7),
        ("temp", PathCategory::Temp, 0.6),
        ("tmp", PathCategory::Temp, 0.6),
        ("target", PathCategory::Build, 0.5), // Rust/Maven specific
        ("dist", PathCategory::Build, 0.4),
        ("build", PathCategory::Build, 0.4),
    ];

    for (pattern, category, confidence) in name_patterns {
        if name_lower == *pattern || name_lower.ends_with(pattern) {
            return Some((category.clone(), *confidence));
        }
    }

    // Check for node_modules (special handling)
    if name == "node_modules" {
        // Only suggest cleaning if it looks abandoned (no package.json nearby)
        let parent = path.parent()?;
        if !parent.join("package.json").exists() {
            return Some((PathCategory::Deps, 0.6));
        }
        // Still return but with lower confidence if package.json exists
        return Some((PathCategory::Deps, 0.3));
    }

    // Content-based heuristics (check what's inside)
    if let Ok(entries) = std::fs::read_dir(path) {
        let entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();

        if entries.len() > 3 {
            // Directory full of timestamped subdirectories = likely cache
            let timestamped_count = entries
                .iter()
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .chars()
                        .all(|c| c.is_numeric() || c == '-' || c == '_' || c == '.')
                })
                .count();

            if timestamped_count > entries.len() / 2 && entries.len() > 5 {
                return Some((PathCategory::Cache, 0.6));
            }

            // Directory full of .log files = logs
            let log_count = entries
                .iter()
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "log")
                        .unwrap_or(false)
                })
                .count();

            if log_count > entries.len() / 2 {
                return Some((PathCategory::Logs, 0.7));
            }
        }
    }

    None
}

/// Calculate a risk score for cleaning a directory.
/// Lower scores = safer to clean.
pub fn calculate_risk_score(path: &Path, category: &PathCategory) -> u8 {
    let base_risk = match category {
        PathCategory::Cache => 10,   // Very safe
        PathCategory::Logs => 20,    // Safe
        PathCategory::Temp => 15,    // Safe
        PathCategory::Build => 30,   // Usually safe, may need rebuild
        PathCategory::Deps => 50,    // Need reinstall
        PathCategory::Archive => 60, // May contain important data
        PathCategory::Other => 70,   // Unknown, be careful
    };

    // Increase risk for paths that look important
    let path_str = path.to_string_lossy().to_lowercase();
    let risk_modifiers: i8 = if path_str.contains("backup") {
        20
    } else if path_str.contains("important") {
        30
    } else if path_str.contains("data") && !path_str.contains("deriveddata") {
        10
    } else {
        0
    };

    (base_risk as i8 + risk_modifiers).clamp(0, 100) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_detect_cache_directories() {
        let path = PathBuf::from("/Users/test/Library/Caches/com.apple.Safari");
        let result = detect_cleanable_directory(&path);
        assert!(result.is_some());
        let (category, confidence) = result.unwrap();
        assert_eq!(category, PathCategory::Cache);
        assert!(confidence > 0.9);
    }

    #[test]
    fn test_detect_logs() {
        let path = PathBuf::from("/Users/test/Library/Logs/DiagnosticReports");
        let result = detect_cleanable_directory(&path);
        assert!(result.is_some());
        let (category, _) = result.unwrap();
        assert_eq!(category, PathCategory::Logs);
    }

    #[test]
    fn test_detect_build_directory() {
        let path = PathBuf::from("/Users/test/Library/Developer/Xcode/DerivedData");
        let result = detect_cleanable_directory(&path);
        assert!(result.is_some());
        let (category, confidence) = result.unwrap();
        assert_eq!(category, PathCategory::Build);
        assert!(confidence > 0.9);
    }

    #[test]
    fn test_risk_scores() {
        let cache_path = PathBuf::from("/test/cache");
        let deps_path = PathBuf::from("/test/deps");

        let cache_risk = calculate_risk_score(&cache_path, &PathCategory::Cache);
        let deps_risk = calculate_risk_score(&deps_path, &PathCategory::Deps);

        assert!(cache_risk < deps_risk);
    }
}
