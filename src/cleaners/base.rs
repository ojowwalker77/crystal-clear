//! Base cleaner implementation with common functionality.

use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::audit::AuditLogger;
use crate::cleaners::traits::*;
use crate::safety::SafeBoundary;
use crate::scanner::{scan_directory, ScanOptions};
use crate::trash::TrashMover;

/// Base cleaner that provides common scanning and cleaning logic.
pub struct BaseCleaner {
    trash_mover: TrashMover,
    audit_logger: AuditLogger,
}

impl BaseCleaner {
    /// Create a new base cleaner.
    pub fn new(trash_mover: TrashMover, audit_logger: AuditLogger) -> Self {
        Self {
            trash_mover,
            audit_logger,
        }
    }

    /// Scan directories for cleanable items.
    pub fn scan_dirs(
        &self,
        cleaner_id: &str,
        roots: &[PathBuf],
        boundary: &SafeBoundary,
        options: &ScanOptions,
        ctx: &CleanerContext,
    ) -> ScanResult {
        let start = Instant::now();
        let mut items = Vec::new();
        let mut errors = Vec::new();

        for root in roots {
            if !root.exists() {
                continue;
            }

            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                scan_directory(root, boundary, options)
            })) {
                Ok(scanned) => {
                    for item in scanned {
                        // Apply age filter
                        if ctx.keep_recent_days > 0 {
                            if let Some(age) = item.age_days {
                                if age < ctx.keep_recent_days {
                                    continue;
                                }
                            }
                        }

                        items.push(CleanableItem {
                            path: item.path.clone(),
                            size: item.size,
                            item_type: if item.is_dir {
                                ItemType::Directory
                            } else if item.is_symlink {
                                ItemType::Symlink
                            } else {
                                ItemType::File
                            },
                            age_days: item.age_days,
                            description: describe_item(&item.path),
                            requires_force: false,
                            risk_level: RiskLevel::Low,
                        });
                    }
                }
                Err(e) => {
                    errors.push(format!("Error scanning {}: {:?}", root.display(), e));
                }
            }
        }

        let total_size: u64 = items.iter().map(|i| i.size).sum();

        ScanResult {
            category: cleaner_id.to_string(),
            items,
            total_size,
            scan_duration: start.elapsed(),
            errors,
        }
    }

    /// Clean items by moving them to trash.
    pub fn clean_items(
        &self,
        cleaner_id: &str,
        items: &[CleanableItem],
        boundary: &SafeBoundary,
        ctx: &CleanerContext,
    ) -> CleanResult {
        let start = Instant::now();
        let mut cleaned = 0;
        let mut bytes_freed = 0u64;
        let mut failed = Vec::new();

        for item in items {
            // Skip items that require force if not forced
            if item.requires_force && !ctx.force {
                failed.push((item.path.clone(), "Requires --force flag".to_string()));
                continue;
            }

            if ctx.dry_run {
                // IMPORTANT: Validate even in dry-run mode to ensure accurate preview
                let validation = self.trash_mover.validator().validate(&item.path, boundary);
                match validation {
                    crate::safety::ValidationResult::Safe => {
                        let _ = self
                            .audit_logger
                            .log_dry_run(cleaner_id, &item.path, item.size);
                        cleaned += 1;
                        bytes_freed += item.size;
                    }
                    crate::safety::ValidationResult::UserExcluded => {
                        // Silently skip excluded items
                    }
                    _ => {
                        failed.push((item.path.clone(), format!("Would fail: {:?}", validation)));
                    }
                }
                continue;
            }

            // Actually move to trash
            match self
                .trash_mover
                .move_to_trash_validated(&item.path, boundary)
            {
                Ok(_) => {
                    let _ = self
                        .audit_logger
                        .log_cleaned(cleaner_id, &item.path, item.size);
                    cleaned += 1;
                    bytes_freed += item.size;
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    let _ = self
                        .audit_logger
                        .log_error(cleaner_id, &item.path, &error_msg);
                    failed.push((item.path.clone(), error_msg));
                }
            }
        }

        CleanResult {
            category: cleaner_id.to_string(),
            items_cleaned: cleaned,
            bytes_freed,
            items_failed: failed,
            clean_duration: start.elapsed(),
        }
    }
}

/// Generate a description for an item based on its path.
fn describe_item(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string()
}
