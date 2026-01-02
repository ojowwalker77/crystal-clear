//! FFI-safe type definitions.
//!
//! These types mirror the internal Rust types but are designed for FFI:
//! - PathBuf becomes String
//! - Duration becomes u64 (milliseconds)
//! - All types implement uniffi traits

/// FFI-safe item type enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FFIItemType {
    File,
    Directory,
    Symlink,
}

/// FFI-safe risk level enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FFIRiskLevel {
    Low,
    Medium,
    High,
}

/// FFI-safe cleaner category enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FFICleanerCategory {
    System,
    Developer,
    Apps,
}

/// Options for scanning operations.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FFIScanOptions {
    pub dry_run: bool,
    pub keep_recent_days: u32,
    pub min_size: Option<u64>,
    pub max_items: Option<u32>,
}

impl Default for FFIScanOptions {
    fn default() -> Self {
        Self {
            dry_run: true,
            keep_recent_days: 0,
            min_size: None,
            max_items: None,
        }
    }
}

/// Options for cleaning operations.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FFICleanOptions {
    pub dry_run: bool,
    pub force: bool,
}

impl Default for FFICleanOptions {
    fn default() -> Self {
        Self {
            dry_run: false,
            force: false,
        }
    }
}

/// FFI-safe cleanable item.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FFICleanableItem {
    pub path: String,
    pub size: u64,
    pub item_type: FFIItemType,
    pub age_days: Option<u32>,
    pub description: String,
    pub requires_force: bool,
    pub risk_level: FFIRiskLevel,
}

/// FFI-safe scan result.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FFIScanResult {
    pub category: String,
    pub category_name: String,
    pub items: Vec<FFICleanableItem>,
    pub total_size: u64,
    pub scan_duration_ms: u64,
    pub errors: Vec<String>,
}

/// Represents a failure during cleaning.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FFICleanFailure {
    pub path: String,
    pub reason: String,
}

/// FFI-safe clean result.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FFICleanResult {
    pub category: String,
    pub items_cleaned: u32,
    pub bytes_freed: u64,
    pub failures: Vec<FFICleanFailure>,
    pub clean_duration_ms: u64,
}

/// FFI-safe installed app information.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FFIInstalledApp {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub bundle_id: Option<String>,
    pub leftover_paths: Vec<String>,
    pub leftover_size: u64,
}

/// FFI-safe disk information.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FFIDiskInfo {
    pub name: String,
    pub mount_point: String,
    pub device_node: String,
    pub filesystem: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
}

/// Information about an available cleaner.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FFICleanerInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: FFICleanerCategory,
    pub available: bool,
}

// ============================================================================
// Large Files Scanner Types
// ============================================================================

/// Options for scanning large files.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FFILargeFileScanOptions {
    /// Minimum file size in bytes (default: 100MB).
    pub min_size_bytes: u64,
    /// Maximum number of results to return.
    pub max_results: u32,
    /// Include hidden files/directories.
    pub include_hidden: bool,
}

impl Default for FFILargeFileScanOptions {
    fn default() -> Self {
        Self {
            min_size_bytes: 100 * 1024 * 1024, // 100MB
            max_results: 100,
            include_hidden: false,
        }
    }
}

// ============================================================================
// Duplicates Scanner Types
// ============================================================================

/// Options for scanning duplicates.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FFIDuplicateScanOptions {
    /// Minimum file size to consider (default: 1MB).
    pub min_size_bytes: u64,
    /// Maximum number of groups to return.
    pub max_groups: u32,
}

impl Default for FFIDuplicateScanOptions {
    fn default() -> Self {
        Self {
            min_size_bytes: 1024 * 1024, // 1MB
            max_groups: 50,
        }
    }
}

/// A group of duplicate files.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FFIDuplicateGroup {
    /// Hash identifying this group.
    pub hash: String,
    /// Size of each file in the group.
    pub size: u64,
    /// Total wasted space (size * (count - 1)).
    pub wasted_size: u64,
    /// Paths of duplicate files.
    pub paths: Vec<String>,
    /// Number of duplicates.
    pub count: u32,
}

/// Result of duplicate scanning.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FFIDuplicateScanResult {
    /// Groups of duplicate files.
    pub groups: Vec<FFIDuplicateGroup>,
    /// Total wasted space across all groups.
    pub total_wasted: u64,
    /// Total files scanned.
    pub files_scanned: u64,
}

// ============================================================================
// Downloads Scanner Types
// ============================================================================

/// Category of a download item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FFIDownloadCategory {
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

/// Options for scanning downloads.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FFIDownloadsScanOptions {
    /// Age threshold for "old" files in days.
    pub old_threshold_days: u32,
    /// Include hidden files.
    pub include_hidden: bool,
}

impl Default for FFIDownloadsScanOptions {
    fn default() -> Self {
        Self {
            old_threshold_days: 30,
            include_hidden: false,
        }
    }
}

/// Size by category entry.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FFICategorySize {
    pub category: FFIDownloadCategory,
    pub size: u64,
}

/// Result of downloads scan.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FFIDownloadsScanResult {
    /// All download items found.
    pub items: Vec<FFICleanableItem>,
    /// Total size of all items.
    pub total_size: u64,
    /// Size by category.
    pub size_by_category: Vec<FFICategorySize>,
}

/// FFI error type.
#[derive(Debug, Clone, uniffi::Error, thiserror::Error)]
pub enum FFIError {
    #[error("Protected path: {path}")]
    ProtectedPath { path: String },

    #[error("Boundary escape: {path} -> {resolved}")]
    BoundaryEscape { path: String, resolved: String },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },

    #[error("Path not found: {path}")]
    PathNotFound { path: String },

    #[error("Trash failed: {path} - {reason}")]
    TrashFailed { path: String, reason: String },

    #[error("Scan failed: {message}")]
    ScanFailed { message: String },

    #[error("Clean failed: {message}")]
    CleanFailed { message: String },

    #[error("Config error: {message}")]
    ConfigError { message: String },
}
