//! Duplicate file scanner.
//!
//! Uses partial hashing (first 4KB + last 4KB + size) for fast duplicate detection.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use blake3::Hasher;
use walkdir::WalkDir;

use crate::safety::is_protected;

/// A group of duplicate files.
#[derive(Debug, Clone)]
pub struct DuplicateGroup {
    /// Hash identifying this group.
    pub hash: String,
    /// Size of each file in the group.
    pub size: u64,
    /// Total wasted space (size * (count - 1)).
    pub wasted_size: u64,
    /// Paths of duplicate files.
    pub paths: Vec<PathBuf>,
    /// Number of duplicates.
    pub count: u32,
}

/// Options for scanning duplicates.
#[derive(Debug, Clone)]
pub struct DuplicateScanOptions {
    /// Minimum file size to consider (default: 1MB).
    pub min_size_bytes: u64,
    /// Maximum number of groups to return.
    pub max_groups: u32,
    /// Include hidden files.
    pub include_hidden: bool,
}

impl Default for DuplicateScanOptions {
    fn default() -> Self {
        Self {
            min_size_bytes: 1024 * 1024, // 1MB
            max_groups: 50,
            include_hidden: false,
        }
    }
}

/// Result of duplicate scanning.
#[derive(Debug)]
pub struct DuplicateScanResult {
    /// Groups of duplicate files.
    pub groups: Vec<DuplicateGroup>,
    /// Total wasted space across all groups.
    pub total_wasted: u64,
    /// Total files scanned.
    pub files_scanned: u64,
}

/// Scan for duplicate files in user directories.
pub fn scan_duplicates(options: &DuplicateScanOptions) -> DuplicateScanResult {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            return DuplicateScanResult {
                groups: vec![],
                total_wasted: 0,
                files_scanned: 0,
            }
        }
    };

    // Directories to scan
    let scan_dirs = [
        home.join("Downloads"),
        home.join("Documents"),
        home.join("Desktop"),
        home.join("Pictures"),
        home.join("Movies"),
    ];

    // Phase 1: Group files by size
    let mut size_groups: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    let mut files_scanned = 0u64;

    for dir in &scan_dirs {
        if !dir.exists() {
            continue;
        }

        collect_files_by_size(dir, options, &mut size_groups, &mut files_scanned);
    }

    // Keep only sizes with multiple files
    size_groups.retain(|_, paths| paths.len() > 1);

    // Phase 2: Hash files with same size
    let mut hash_groups: HashMap<String, DuplicateGroup> = HashMap::new();

    for (_size, paths) in size_groups {
        // Hash each file
        let mut path_hashes: HashMap<String, Vec<PathBuf>> = HashMap::new();

        for path in paths {
            if let Some(hash) = compute_partial_hash(&path) {
                path_hashes.entry(hash).or_default().push(path);
            }
        }

        // Add groups with duplicates
        for (hash, dup_paths) in path_hashes {
            if dup_paths.len() > 1 {
                let size = std::fs::metadata(&dup_paths[0])
                    .map(|m| m.len())
                    .unwrap_or(0);
                let count = dup_paths.len() as u32;
                let wasted = size * (count as u64 - 1);

                hash_groups.insert(
                    hash.clone(),
                    DuplicateGroup {
                        hash,
                        size,
                        wasted_size: wasted,
                        paths: dup_paths,
                        count,
                    },
                );
            }
        }
    }

    // Sort by wasted space descending
    let mut groups: Vec<DuplicateGroup> = hash_groups.into_values().collect();
    groups.sort_by(|a, b| b.wasted_size.cmp(&a.wasted_size));

    // Take top N groups
    groups.truncate(options.max_groups as usize);

    let total_wasted = groups.iter().map(|g| g.wasted_size).sum();

    DuplicateScanResult {
        groups,
        total_wasted,
        files_scanned,
    }
}

fn collect_files_by_size(
    dir: &Path,
    options: &DuplicateScanOptions,
    size_groups: &mut HashMap<u64, Vec<PathBuf>>,
    files_scanned: &mut u64,
) {
    // Skip certain directories
    let skip_dirs = [
        "Library",
        ".Trash",
        ".cache",
        "node_modules",
        ".git",
        ".cargo",
        ".rustup",
        "target",
        "DerivedData",
    ];

    let walker = WalkDir::new(dir)
        .follow_links(false)
        .max_depth(8)
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

            if entry.file_type().is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if skip_dirs.iter().any(|d| d == &name) {
                        return false;
                    }
                }
            }

            true
        });

    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();

        // Only files
        if !entry.file_type().is_file() {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let size = metadata.len();
        *files_scanned += 1;

        // Skip small files
        if size < options.min_size_bytes {
            continue;
        }

        size_groups
            .entry(size)
            .or_default()
            .push(path.to_path_buf());
    }
}

/// Compute a partial hash of a file for fast comparison.
/// Uses first 4KB + last 4KB + file size.
fn compute_partial_hash(path: &Path) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let metadata = file.metadata().ok()?;
    let size = metadata.len();

    let mut hasher = Hasher::new();

    // Add file size to hash
    hasher.update(&size.to_le_bytes());

    // Read first 4KB
    let mut buffer = [0u8; 4096];
    let bytes_read = file.read(&mut buffer).ok()?;
    hasher.update(&buffer[..bytes_read]);

    // If file is larger than 8KB, read last 4KB
    if size > 8192 {
        file.seek(SeekFrom::End(-4096)).ok()?;
        let bytes_read = file.read(&mut buffer).ok()?;
        hasher.update(&buffer[..bytes_read]);
    }

    let hash = hasher.finalize();
    Some(hash.to_hex().to_string())
}

/// Convert DuplicateGroup to FFI-safe format.
impl DuplicateGroup {
    pub fn path_strings(&self) -> Vec<String> {
        self.paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_partial_hash_same_content() {
        let temp = TempDir::new().unwrap();

        // Create two identical files
        let content = vec![0u8; 10000]; // 10KB
        let file1 = temp.path().join("file1.bin");
        let file2 = temp.path().join("file2.bin");
        fs::write(&file1, &content).unwrap();
        fs::write(&file2, &content).unwrap();

        let hash1 = compute_partial_hash(&file1).unwrap();
        let hash2 = compute_partial_hash(&file2).unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_partial_hash_different_content() {
        let temp = TempDir::new().unwrap();

        // Create two different files of same size
        let content1 = vec![0u8; 10000];
        let content2 = vec![1u8; 10000];
        let file1 = temp.path().join("file1.bin");
        let file2 = temp.path().join("file2.bin");
        fs::write(&file1, &content1).unwrap();
        fs::write(&file2, &content2).unwrap();

        let hash1 = compute_partial_hash(&file1).unwrap();
        let hash2 = compute_partial_hash(&file2).unwrap();

        assert_ne!(hash1, hash2);
    }
}
