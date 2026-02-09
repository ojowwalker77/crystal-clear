//! Downloads folder scanner.
//!
//! Analyzes the Downloads folder and categorizes files by type and age.

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::ffi::{FFICleanableItem, FFIItemType, FFIRiskLevel};
use crate::safety::is_protected;

/// Category of a download item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadCategory {
    /// Files older than 30 days
    OldFiles,
    /// Archive files (zip, tar, dmg, pkg, etc.)
    Archives,
    /// Large media files (videos, audio)
    LargeMedia,
    /// Document files
    Documents,
    /// Disk images and installers
    Installers,
    /// Other files
    Other,
}

impl DownloadCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OldFiles => "Old Files (30+ days)",
            Self::Archives => "Archives",
            Self::LargeMedia => "Large Media",
            Self::Documents => "Documents",
            Self::Installers => "Installers",
            Self::Other => "Other",
        }
    }
}

/// A categorized download item.
#[derive(Debug, Clone)]
pub struct DownloadItem {
    pub path: PathBuf,
    pub size: u64,
    pub category: DownloadCategory,
    pub age_days: Option<u32>,
    pub is_dir: bool,
}

/// Result of downloads scan.
#[derive(Debug)]
pub struct DownloadsScanResult {
    /// All download items found.
    pub items: Vec<DownloadItem>,
    /// Total size of all items.
    pub total_size: u64,
    /// Size by category.
    pub size_by_category: [(DownloadCategory, u64); 6],
}

/// Options for scanning downloads.
#[derive(Debug, Clone)]
pub struct DownloadsScanOptions {
    /// Age threshold for "old" files (default: 30 days).
    pub old_threshold_days: u32,
    /// Include hidden files.
    pub include_hidden: bool,
}

impl Default for DownloadsScanOptions {
    fn default() -> Self {
        Self {
            old_threshold_days: 30,
            include_hidden: false,
        }
    }
}

/// Scan the Downloads folder.
pub fn scan_downloads(options: &DownloadsScanOptions) -> DownloadsScanResult {
    let downloads = match dirs::download_dir() {
        Some(d) => d,
        None => {
            // Fallback to ~/Downloads
            match dirs::home_dir() {
                Some(h) => h.join("Downloads"),
                None => {
                    return DownloadsScanResult {
                        items: vec![],
                        total_size: 0,
                        size_by_category: [
                            (DownloadCategory::OldFiles, 0),
                            (DownloadCategory::Archives, 0),
                            (DownloadCategory::LargeMedia, 0),
                            (DownloadCategory::Documents, 0),
                            (DownloadCategory::Installers, 0),
                            (DownloadCategory::Other, 0),
                        ],
                    }
                }
            }
        }
    };

    if !downloads.exists() {
        return DownloadsScanResult {
            items: vec![],
            total_size: 0,
            size_by_category: [
                (DownloadCategory::OldFiles, 0),
                (DownloadCategory::Archives, 0),
                (DownloadCategory::LargeMedia, 0),
                (DownloadCategory::Documents, 0),
                (DownloadCategory::Installers, 0),
                (DownloadCategory::Other, 0),
            ],
        };
    }

    let mut items = Vec::new();
    let mut total_size = 0u64;
    let mut old_files_size = 0u64;
    let mut archives_size = 0u64;
    let mut media_size = 0u64;
    let mut documents_size = 0u64;
    let mut installers_size = 0u64;
    let mut other_size = 0u64;

    let walker = WalkDir::new(&downloads)
        .follow_links(false)
        .min_depth(1)
        .max_depth(1)
        .into_iter();

    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();

        if is_protected(path) {
            continue;
        }

        // Skip hidden files if not requested
        if !options.include_hidden {
            if let Some(name) = path.file_name() {
                if name.to_string_lossy().starts_with('.') {
                    continue;
                }
            }
        }

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let size = if metadata.is_file() {
            metadata.len()
        } else if metadata.is_dir() {
            // For directories, calculate total size
            calculate_dir_size(path)
        } else {
            continue;
        };

        let age_days = metadata
            .modified()
            .ok()
            .and_then(|t| t.elapsed().ok().map(|d| (d.as_secs() / 86400) as u32));

        // Determine category
        let category = categorize_download(path, size, age_days, options.old_threshold_days);

        total_size += size;
        match category {
            DownloadCategory::OldFiles => old_files_size += size,
            DownloadCategory::Archives => archives_size += size,
            DownloadCategory::LargeMedia => media_size += size,
            DownloadCategory::Documents => documents_size += size,
            DownloadCategory::Installers => installers_size += size,
            DownloadCategory::Other => other_size += size,
        }

        items.push(DownloadItem {
            path: path.to_path_buf(),
            size,
            category,
            age_days,
            is_dir: metadata.is_dir(),
        });
    }

    // Sort by size descending
    items.sort_by(|a, b| b.size.cmp(&a.size));

    DownloadsScanResult {
        items,
        total_size,
        size_by_category: [
            (DownloadCategory::OldFiles, old_files_size),
            (DownloadCategory::Archives, archives_size),
            (DownloadCategory::LargeMedia, media_size),
            (DownloadCategory::Documents, documents_size),
            (DownloadCategory::Installers, installers_size),
            (DownloadCategory::Other, other_size),
        ],
    }
}

fn categorize_download(
    path: &Path,
    size: u64,
    age_days: Option<u32>,
    old_threshold: u32,
) -> DownloadCategory {
    let extension = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    // Check for installers first (high priority category)
    if ["dmg", "pkg", "iso", "app", "mpkg"].contains(&extension.as_str()) {
        return DownloadCategory::Installers;
    }

    // Check for archives
    if ["zip", "tar", "gz", "bz2", "xz", "rar", "7z", "tgz", "tbz2"].contains(&extension.as_str()) {
        return DownloadCategory::Archives;
    }

    // Check for large media (> 50MB)
    if size > 50 * 1024 * 1024 {
        if ["mp4", "mov", "avi", "mkv", "webm", "m4v", "wmv", "flv"].contains(&extension.as_str()) {
            return DownloadCategory::LargeMedia;
        }
        if ["mp3", "wav", "flac", "m4a", "aac", "ogg", "wma"].contains(&extension.as_str()) {
            return DownloadCategory::LargeMedia;
        }
    }

    // Check for documents
    if [
        "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "txt", "rtf", "odt", "ods", "odp",
    ]
    .contains(&extension.as_str())
    {
        return DownloadCategory::Documents;
    }

    // Check for old files
    if let Some(days) = age_days {
        if days >= old_threshold {
            return DownloadCategory::OldFiles;
        }
    }

    DownloadCategory::Other
}

fn calculate_dir_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

/// Convert DownloadItem to FFI format.
impl DownloadItem {
    pub fn to_ffi(&self) -> FFICleanableItem {
        let risk = match self.category {
            DownloadCategory::OldFiles => FFIRiskLevel::Low,
            DownloadCategory::Archives => FFIRiskLevel::Low,
            DownloadCategory::Installers => FFIRiskLevel::Low,
            DownloadCategory::LargeMedia => FFIRiskLevel::Low,
            DownloadCategory::Documents => FFIRiskLevel::Medium,
            DownloadCategory::Other => FFIRiskLevel::Medium,
        };

        let description = format!(
            "{} - {}",
            self.category.as_str(),
            bytesize::ByteSize(self.size)
        );

        FFICleanableItem {
            path: self.path.to_string_lossy().to_string(),
            size: self.size,
            item_type: if self.is_dir {
                FFIItemType::Directory
            } else {
                FFIItemType::File
            },
            age_days: self.age_days,
            description,
            requires_force: false,
            risk_level: risk,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_installer() {
        let path = PathBuf::from("/Downloads/App.dmg");
        let cat = categorize_download(&path, 1000, Some(5), 30);
        assert_eq!(cat, DownloadCategory::Installers);
    }

    #[test]
    fn test_categorize_archive() {
        let path = PathBuf::from("/Downloads/files.zip");
        let cat = categorize_download(&path, 1000, Some(5), 30);
        assert_eq!(cat, DownloadCategory::Archives);
    }

    #[test]
    fn test_categorize_old_file() {
        let path = PathBuf::from("/Downloads/random.xyz");
        let cat = categorize_download(&path, 1000, Some(60), 30);
        assert_eq!(cat, DownloadCategory::OldFiles);
    }
}
