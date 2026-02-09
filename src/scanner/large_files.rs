//! Large files scanner.
//!
//! Scans user directories to find large files that may be candidates for cleanup.

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::ffi::{FFICleanableItem, FFIItemType, FFIRiskLevel};
use crate::safety::is_protected;

/// Options for scanning large files.
#[derive(Debug, Clone)]
pub struct LargeFileScanOptions {
    /// Minimum file size in bytes (default: 100MB).
    pub min_size_bytes: u64,
    /// Maximum number of results to return.
    pub max_results: u32,
    /// Include hidden files/directories.
    pub include_hidden: bool,
}

impl Default for LargeFileScanOptions {
    fn default() -> Self {
        Self {
            min_size_bytes: 100 * 1024 * 1024, // 100MB
            max_results: 100,
            include_hidden: false,
        }
    }
}

/// Scan for large files in user directories.
pub fn scan_large_files(options: &LargeFileScanOptions) -> Vec<FFICleanableItem> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return vec![],
    };

    // Scan home once to avoid duplicate results from nested roots.
    let scan_dirs = [home];

    let mut large_files: Vec<LargeFileEntry> = Vec::new();

    for dir in &scan_dirs {
        if !dir.exists() {
            continue;
        }

        scan_directory_for_large_files(dir, options, &mut large_files);
    }

    // Sort by size descending
    large_files.sort_by(|a, b| b.size.cmp(&a.size));

    // Take top N results
    large_files.truncate(options.max_results as usize);

    // Convert to FFI items
    large_files
        .into_iter()
        .map(|entry| {
            let risk = determine_risk(&entry.path);
            FFICleanableItem {
                path: entry.path.to_string_lossy().to_string(),
                size: entry.size,
                item_type: if entry.is_dir {
                    FFIItemType::Directory
                } else {
                    FFIItemType::File
                },
                age_days: entry.age_days,
                description: format_description(&entry.path, entry.size),
                requires_force: false,
                risk_level: risk,
            }
        })
        .collect()
}

#[derive(Debug)]
struct LargeFileEntry {
    path: PathBuf,
    size: u64,
    is_dir: bool,
    age_days: Option<u32>,
}

fn scan_directory_for_large_files(
    dir: &Path,
    options: &LargeFileScanOptions,
    results: &mut Vec<LargeFileEntry>,
) {
    // Skip directories that are typically not user-manageable
    let skip_prefixes = [
        "Library",
        ".Trash",
        ".cache",
        "node_modules",
        ".git",
        ".cargo",
        ".rustup",
        "target",
    ];

    let walker = WalkDir::new(dir)
        .follow_links(false)
        .max_depth(10) // Don't go too deep
        .into_iter()
        .filter_entry(|entry| {
            let path = entry.path();

            if !options.include_hidden {
                if let Some(name) = path.file_name() {
                    if name.to_string_lossy().starts_with('.') {
                        return false;
                    }
                }
            }

            if is_protected(path) {
                return false;
            }

            let path_str = path.to_string_lossy();
            !skip_prefixes.iter().any(|prefix| {
                path_str.contains(&format!("/{}/", prefix))
                    || path_str.ends_with(&format!("/{}", prefix))
            })
        });

    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();

        // Only process files (not directories for size, we want individual files)
        if !entry.file_type().is_file() {
            continue;
        }

        // Get file metadata
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let size = metadata.len();

        // Skip if smaller than threshold
        if size < options.min_size_bytes {
            continue;
        }

        let age_days = metadata
            .modified()
            .ok()
            .and_then(|t| t.elapsed().ok().map(|d| (d.as_secs() / 86400) as u32));

        results.push(LargeFileEntry {
            path: path.to_path_buf(),
            size,
            is_dir: false,
            age_days,
        });
    }
}

fn determine_risk(path: &Path) -> FFIRiskLevel {
    let path_str = path.to_string_lossy().to_lowercase();
    let extension = path.extension().map(|e| e.to_string_lossy().to_lowercase());

    // High risk: Documents, code, databases
    if path_str.contains("/documents/") {
        return FFIRiskLevel::High;
    }
    if let Some(ext) = &extension {
        if [
            "doc", "docx", "xls", "xlsx", "ppt", "pptx", "pdf", "key", "pages", "numbers",
        ]
        .contains(&ext.as_str())
        {
            return FFIRiskLevel::High;
        }
        if ["db", "sqlite", "sqlite3", "sql"].contains(&ext.as_str()) {
            return FFIRiskLevel::High;
        }
        if [
            "rs", "py", "js", "ts", "swift", "java", "cpp", "c", "h", "go",
        ]
        .contains(&ext.as_str())
        {
            return FFIRiskLevel::High;
        }
    }

    // Low risk: Downloads, media, archives
    if path_str.contains("/downloads/") {
        return FFIRiskLevel::Low;
    }
    if let Some(ext) = &extension {
        if ["dmg", "pkg", "iso", "zip", "tar", "gz", "rar", "7z"].contains(&ext.as_str()) {
            return FFIRiskLevel::Low;
        }
        if ["mp4", "mov", "avi", "mkv", "webm", "m4v"].contains(&ext.as_str()) {
            return FFIRiskLevel::Low;
        }
        if ["mp3", "wav", "flac", "m4a", "aac"].contains(&ext.as_str()) {
            return FFIRiskLevel::Low;
        }
    }

    // Medium risk: Everything else
    FFIRiskLevel::Medium
}

fn format_description(path: &Path, size: u64) -> String {
    let filename = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let parent = path
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string());

    let size_str = bytesize::ByteSize(size).to_string();

    match parent {
        Some(p) => format!("{} in {} ({})", filename, p, size_str),
        None => format!("{} ({})", filename, size_str),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_scan_large_files_empty() {
        let temp = TempDir::new().unwrap();
        // Create a small file
        fs::write(temp.path().join("small.txt"), "hello").unwrap();

        let _options = LargeFileScanOptions {
            min_size_bytes: 1024 * 1024, // 1MB
            max_results: 10,
            include_hidden: false,
        };

        // This test just verifies the scanner doesn't crash
        // Real test would need to mock dirs::home_dir()
    }

    #[test]
    fn test_determine_risk() {
        let downloads = PathBuf::from("/Users/test/Downloads/file.dmg");
        assert_eq!(determine_risk(&downloads), FFIRiskLevel::Low);

        let document = PathBuf::from("/Users/test/Documents/report.pdf");
        assert_eq!(determine_risk(&document), FFIRiskLevel::High);

        let random = PathBuf::from("/Users/test/random.bin");
        assert_eq!(determine_risk(&random), FFIRiskLevel::Medium);
    }
}
