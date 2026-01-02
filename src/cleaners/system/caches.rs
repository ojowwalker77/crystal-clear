//! System caches cleaner.

use std::path::PathBuf;

use crate::audit::AuditLogger;
use crate::cleaners::base::BaseCleaner;
use crate::cleaners::traits::*;
use crate::config::schema::CategoryConfig;
use crate::safety::boundaries::presets;
use crate::safety::SafeBoundary;
use crate::scanner::ScanOptions;
use crate::trash::TrashMover;

/// Cleaner for user cache directories.
pub struct SystemCachesCleaner {
    base: BaseCleaner,
    paths: Vec<PathBuf>,
    exclude: Vec<PathBuf>,
}

impl SystemCachesCleaner {
    pub fn new(
        trash_mover: TrashMover,
        audit_logger: AuditLogger,
        config: &CategoryConfig,
    ) -> Self {
        let home = dirs::home_dir().expect("Home directory required");
        let mut paths = vec![home.join("Library/Caches")];

        // Add custom paths from config
        paths.extend(config.paths.iter().cloned());

        Self {
            base: BaseCleaner::new(trash_mover, audit_logger),
            paths,
            exclude: config.exclude.clone(),
        }
    }
}

impl Cleaner for SystemCachesCleaner {
    fn id(&self) -> &'static str {
        "system-cache"
    }

    fn name(&self) -> &'static str {
        "System Caches"
    }

    fn description(&self) -> &'static str {
        "User application caches (~/Library/Caches)"
    }

    fn category(&self) -> CleanerCategory {
        CleanerCategory::System
    }

    fn safe_boundary(&self) -> SafeBoundary {
        presets::user_caches()
    }

    fn is_available(&self) -> bool {
        self.paths.iter().any(|p| p.exists())
    }

    fn scan(&self, ctx: &CleanerContext) -> ScanResult {
        let options = ScanOptions {
            top_level_only: true, // Only clean top-level cache directories
            directories_only: true,
            ..Default::default()
        };

        let mut result = self.base.scan_dirs(
            self.id(),
            &self.paths,
            &self.safe_boundary(),
            &options,
            ctx,
        );

        // Filter out excluded paths
        result.items.retain(|item| {
            !self.exclude.iter().any(|excl| {
                item.path.starts_with(excl) || item.path == *excl
            })
        });

        // Recalculate total size
        result.total_size = result.items.iter().map(|i| i.size).sum();

        result
    }

    fn clean(&self, items: &[CleanableItem], ctx: &CleanerContext) -> CleanResult {
        self.base.clean_items(self.id(), items, &self.safe_boundary(), ctx)
    }
}
