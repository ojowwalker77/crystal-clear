//! Directory walking utilities.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use walkdir::{DirEntry, WalkDir};

use crate::safety::{is_protected, PathValidator, SafeBoundary, ValidationResult};
use crate::scanner::size::calculate_size;

/// Options for scanning directories.
#[derive(Debug, Clone)]
pub struct ScanOptions {
    /// Maximum depth to traverse (None = unlimited).
    pub max_depth: Option<usize>,
    /// Minimum file age in days.
    pub min_age_days: Option<u32>,
    /// Minimum file size in bytes.
    pub min_size: Option<u64>,
    /// Only include directories (not individual files).
    pub directories_only: bool,
    /// Only scan top-level items.
    pub top_level_only: bool,
    /// Follow symlinks (NOT recommended for safety).
    pub follow_links: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            max_depth: None,
            min_age_days: None,
            min_size: None,
            directories_only: false,
            top_level_only: false,
            follow_links: false, // NEVER follow symlinks by default
        }
    }
}

/// A scanned item with metadata.
#[derive(Debug, Clone)]
pub struct ScannedItem {
    pub path: PathBuf,
    pub size: u64,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub age_days: Option<u32>,
    pub modified: Option<SystemTime>,
}

impl ScannedItem {
    fn from_entry(
        entry: &DirEntry,
        metadata: &std::fs::Metadata,
        size: u64,
        age_days: Option<u32>,
    ) -> Self {
        Self {
            path: entry.path().to_path_buf(),
            size,
            is_dir: metadata.is_dir(),
            is_symlink: entry.path_is_symlink(),
            age_days,
            modified: metadata.modified().ok(),
        }
    }
}

/// Scan a directory for items matching the options.
pub fn scan_directory(
    root: &Path,
    boundary: &SafeBoundary,
    options: &ScanOptions,
) -> Vec<ScannedItem> {
    let validator = PathValidator::default_validator();
    let mut items = Vec::new();

    let walker = WalkDir::new(root)
        .follow_links(options.follow_links)
        .max_depth(options.max_depth.unwrap_or(usize::MAX))
        .min_depth(1)
        .into_iter()
        .filter_entry(|entry| {
            let path = entry.path();
            if path == root {
                return true;
            }

            if is_protected(path) {
                return false;
            }

            if options.top_level_only && entry.depth() > 1 {
                return false;
            }

            matches!(validator.validate(path, boundary), ValidationResult::Safe)
        });

    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();

        // Skip if only wanting directories
        if options.directories_only && !entry.file_type().is_dir() {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };

        let age_days = metadata
            .modified()
            .ok()
            .and_then(|t| t.elapsed().ok().map(|d| (d.as_secs() / 86400) as u32));

        if let Some(min_age) = options.min_age_days {
            if age_days.map(|a| a < min_age).unwrap_or(true) {
                continue;
            }
        }

        let size = if metadata.is_file() {
            metadata.len()
        } else if metadata.is_dir() {
            calculate_size(path)
        } else {
            0
        };

        if let Some(min_size) = options.min_size {
            if size < min_size {
                continue;
            }
        }

        items.push(ScannedItem::from_entry(&entry, &metadata, size, age_days));
    }

    items
}

/// Scan multiple directories and combine results.
pub fn scan_directories(
    roots: &[PathBuf],
    boundary: &SafeBoundary,
    options: &ScanOptions,
) -> Vec<ScannedItem> {
    roots
        .iter()
        .filter(|p| p.exists())
        .flat_map(|root| scan_directory(root, boundary, options))
        .collect()
}

/// Calculate total size of scanned items.
pub fn total_size(items: &[ScannedItem]) -> u64 {
    items.iter().map(|i| i.size).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_scan_directory() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        // Create some test files
        fs::write(root.join("file1.txt"), "hello").unwrap();
        fs::write(root.join("file2.txt"), "world").unwrap();
        let subdir = root.join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("file3.txt"), "test").unwrap();

        let boundary = SafeBoundary::new(vec![root.clone()], "test");
        let options = ScanOptions::default();

        let items = scan_directory(&root, &boundary, &options);

        // Should find all items (2 files + 1 dir + 1 file in subdir)
        assert!(!items.is_empty());
    }

    #[test]
    fn test_scan_top_level_only() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        fs::write(root.join("file1.txt"), "hello").unwrap();
        let subdir = root.join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("file2.txt"), "world").unwrap();

        let boundary = SafeBoundary::new(vec![root.clone()], "test");
        let options = ScanOptions {
            top_level_only: true,
            ..Default::default()
        };

        let items = scan_directory(&root, &boundary, &options);

        // Should only find top-level items
        for item in &items {
            assert_eq!(item.path.parent(), Some(root.as_path()));
        }
    }

    #[test]
    fn test_scan_directories_only() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();

        fs::write(root.join("file1.txt"), "hello").unwrap();
        let subdir = root.join("subdir");
        fs::create_dir(&subdir).unwrap();

        let boundary = SafeBoundary::new(vec![root.clone()], "test");
        let options = ScanOptions {
            directories_only: true,
            ..Default::default()
        };

        let items = scan_directory(&root, &boundary, &options);

        // All items should be directories
        for item in &items {
            assert!(item.is_dir);
        }
    }
}
