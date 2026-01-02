//! Cleaner trait definitions.

use std::path::PathBuf;
use std::time::Duration;

use crate::safety::SafeBoundary;

/// Risk level for a cleanable item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    /// Safe to delete, automatically regenerated (caches, logs).
    Low,
    /// Safe but may require app restart or redownload.
    Medium,
    /// Check with user, may affect app state or contain data.
    High,
}

impl RiskLevel {
    /// Get a display string for the risk level.
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Low => "low",
            RiskLevel::Medium => "medium",
            RiskLevel::High => "high",
        }
    }
}

/// Type of cleanable item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemType {
    File,
    Directory,
    Symlink,
}

/// A single cleanable item discovered by a cleaner.
#[derive(Debug, Clone)]
pub struct CleanableItem {
    /// Path to the item.
    pub path: PathBuf,
    /// Size in bytes.
    pub size: u64,
    /// Type of item.
    pub item_type: ItemType,
    /// Age in days (if known).
    pub age_days: Option<u32>,
    /// Human-readable description.
    pub description: String,
    /// Whether this item requires --force to clean.
    pub requires_force: bool,
    /// Risk level of cleaning this item.
    pub risk_level: RiskLevel,
}

/// Result of a scan operation.
#[derive(Debug, Clone)]
pub struct ScanResult {
    /// Cleaner category.
    pub category: String,
    /// Discovered items.
    pub items: Vec<CleanableItem>,
    /// Total size of all items.
    pub total_size: u64,
    /// Time taken to scan.
    pub scan_duration: Duration,
    /// Errors encountered during scan.
    pub errors: Vec<String>,
}

impl ScanResult {
    /// Create an empty scan result.
    pub fn empty(category: &str) -> Self {
        Self {
            category: category.to_string(),
            items: Vec::new(),
            total_size: 0,
            scan_duration: Duration::ZERO,
            errors: Vec::new(),
        }
    }
}

/// Result of a clean operation.
#[derive(Debug)]
pub struct CleanResult {
    /// Cleaner category.
    pub category: String,
    /// Number of items successfully cleaned.
    pub items_cleaned: usize,
    /// Total bytes freed.
    pub bytes_freed: u64,
    /// Items that failed to clean.
    pub items_failed: Vec<(PathBuf, String)>,
    /// Time taken to clean.
    pub clean_duration: Duration,
}

impl CleanResult {
    /// Create an empty clean result.
    pub fn empty(category: &str) -> Self {
        Self {
            category: category.to_string(),
            items_cleaned: 0,
            bytes_freed: 0,
            items_failed: Vec::new(),
            clean_duration: Duration::ZERO,
        }
    }
}

/// Configuration passed to cleaners.
#[derive(Debug, Clone)]
pub struct CleanerContext {
    /// Whether to actually clean or just scan.
    pub dry_run: bool,
    /// Whether to bypass soft protections.
    pub force: bool,
    /// Number of days to keep recent files.
    pub keep_recent_days: u32,
    /// Minimum size to consider for cleaning.
    pub min_size: Option<u64>,
    /// Maximum number of items to process.
    pub max_items: Option<usize>,
    /// Whether to skip confirmation prompts.
    pub skip_confirm: bool,
}

impl Default for CleanerContext {
    fn default() -> Self {
        Self {
            dry_run: true, // Default to dry-run for safety
            force: false,
            keep_recent_days: 0,
            min_size: None,
            max_items: None,
            skip_confirm: false,
        }
    }
}

/// Category of cleaners.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CleanerCategory {
    System,
    Developer,
    Apps,
}

impl CleanerCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            CleanerCategory::System => "system",
            CleanerCategory::Developer => "developer",
            CleanerCategory::Apps => "apps",
        }
    }
}

/// The core trait that all cleaners must implement.
pub trait Cleaner: Send + Sync {
    /// Unique identifier for this cleaner.
    fn id(&self) -> &'static str;

    /// Human-readable name.
    fn name(&self) -> &'static str;

    /// Description of what this cleaner does.
    fn description(&self) -> &'static str;

    /// Category group.
    fn category(&self) -> CleanerCategory;

    /// Define the safe boundary for this cleaner.
    fn safe_boundary(&self) -> SafeBoundary;

    /// Check if this cleaner is available on this system.
    fn is_available(&self) -> bool;

    /// Scan for cleanable items (read-only operation).
    fn scan(&self, ctx: &CleanerContext) -> ScanResult;

    /// Clean the specified items.
    fn clean(&self, items: &[CleanableItem], ctx: &CleanerContext) -> CleanResult;

    /// Quick size estimate without full scan.
    fn estimate_size(&self) -> Option<u64> {
        None
    }
}
