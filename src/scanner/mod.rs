//! Scanner module for discovering cleanable items.

pub mod apps;
pub mod downloads;
pub mod duplicates;
pub mod large_files;
pub mod size;
pub mod walker;

pub use apps::{scan_applications, InstalledApp};
pub use downloads::{scan_downloads, DownloadCategory, DownloadItem, DownloadsScanOptions, DownloadsScanResult};
pub use duplicates::{scan_duplicates, DuplicateGroup, DuplicateScanOptions, DuplicateScanResult};
pub use large_files::{scan_large_files, LargeFileScanOptions};
pub use size::{calculate_size, format_size, get_age_days, parse_size};
pub use walker::{scan_directory, scan_directories, total_size, ScanOptions, ScannedItem};
