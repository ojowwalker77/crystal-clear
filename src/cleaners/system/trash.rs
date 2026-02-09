//! Trash cleaner - empties the user's trash.

use std::path::PathBuf;
use std::time::Instant;

use crate::audit::AuditLogger;
use crate::cleaners::traits::*;
use crate::config::schema::TrashConfig;
use crate::safety::boundaries::presets;
use crate::safety::SafeBoundary;
use crate::scanner::{scan_directory, ScanOptions};

/// Cleaner for the user's trash.
pub struct TrashCleaner {
    audit_logger: AuditLogger,
    trash_path: PathBuf,
    min_age_days: u32,
}

impl TrashCleaner {
    pub fn new(audit_logger: AuditLogger, config: &TrashConfig) -> Self {
        let home = dirs::home_dir().expect("Home directory required");
        Self {
            audit_logger,
            trash_path: home.join(".Trash"),
            min_age_days: config.min_age_days,
        }
    }
}

impl Cleaner for TrashCleaner {
    fn id(&self) -> &'static str {
        "trash"
    }

    fn name(&self) -> &'static str {
        "Trash"
    }

    fn description(&self) -> &'static str {
        "Empty the user's trash (~/.Trash)"
    }

    fn category(&self) -> CleanerCategory {
        CleanerCategory::System
    }

    fn safe_boundary(&self) -> SafeBoundary {
        presets::user_trash()
    }

    fn is_available(&self) -> bool {
        self.trash_path.exists()
    }

    fn scan(&self, ctx: &CleanerContext) -> ScanResult {
        let start = Instant::now();

        if !self.trash_path.exists() {
            return ScanResult::empty(self.id());
        }

        let min_age = if self.min_age_days > 0 {
            Some(self.min_age_days)
        } else if ctx.keep_recent_days > 0 {
            Some(ctx.keep_recent_days)
        } else {
            None
        };

        let options = ScanOptions {
            top_level_only: true,
            min_age_days: min_age,
            ..Default::default()
        };

        let scanned = scan_directory(&self.trash_path, &self.safe_boundary(), &options);

        let items: Vec<CleanableItem> = scanned
            .into_iter()
            .map(|item| CleanableItem {
                path: item.path,
                size: item.size,
                item_type: if item.is_dir {
                    ItemType::Directory
                } else {
                    ItemType::File
                },
                age_days: item.age_days,
                description: "Trash item".to_string(),
                requires_force: false,
                risk_level: RiskLevel::Low,
            })
            .collect();

        let total_size = items.iter().map(|i| i.size).sum();

        ScanResult {
            category: self.id().to_string(),
            items,
            total_size,
            scan_duration: start.elapsed(),
            errors: Vec::new(),
        }
    }

    fn clean(&self, items: &[CleanableItem], ctx: &CleanerContext) -> CleanResult {
        let start = Instant::now();
        let mut cleaned = 0;
        let mut bytes_freed = 0u64;
        let mut failed = Vec::new();
        let boundary = self.safe_boundary();

        for item in items {
            // CRITICAL: Validate path is within trash boundary before any operation
            let canonical = match item.path.canonicalize() {
                Ok(p) => p,
                Err(e) => {
                    failed.push((item.path.clone(), format!("Cannot resolve path: {}", e)));
                    continue;
                }
            };

            if !canonical.starts_with(&self.trash_path) {
                failed.push((
                    item.path.clone(),
                    "Path is outside trash directory - refusing to delete".to_string(),
                ));
                continue;
            }

            // Also check against boundary
            match boundary.contains(&canonical) {
                Ok(true) => {}
                _ => {
                    failed.push((item.path.clone(), "Path fails boundary check".to_string()));
                    continue;
                }
            }

            if ctx.dry_run {
                let _ = self
                    .audit_logger
                    .log_dry_run(self.id(), &item.path, item.size);
                cleaned += 1;
                bytes_freed += item.size;
                continue;
            }

            // For trash cleaning, we actually delete (since items are already in trash)
            let result = if item.path.is_dir() {
                std::fs::remove_dir_all(&item.path)
            } else {
                std::fs::remove_file(&item.path)
            };

            match result {
                Ok(()) => {
                    let _ = self
                        .audit_logger
                        .log_cleaned(self.id(), &item.path, item.size);
                    cleaned += 1;
                    bytes_freed += item.size;
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    let _ = self
                        .audit_logger
                        .log_error(self.id(), &item.path, &error_msg);
                    failed.push((item.path.clone(), error_msg));
                }
            }
        }

        CleanResult {
            category: self.id().to_string(),
            items_cleaned: cleaned,
            bytes_freed,
            items_failed: failed,
            clean_duration: start.elapsed(),
        }
    }
}
