//! npm cache cleaner.

use std::path::PathBuf;

use crate::audit::AuditLogger;
use crate::cleaners::base::BaseCleaner;
use crate::cleaners::traits::*;
use crate::config::schema::CategoryConfig;
use crate::safety::boundaries::presets;
use crate::safety::SafeBoundary;
use crate::scanner::ScanOptions;
use crate::trash::TrashMover;

/// Cleaner for npm cache.
pub struct NpmCleaner {
    base: BaseCleaner,
    cache_path: PathBuf,
}

impl NpmCleaner {
    pub fn new(
        trash_mover: TrashMover,
        audit_logger: AuditLogger,
        _config: &CategoryConfig,
    ) -> Self {
        let home = dirs::home_dir().expect("Home directory required");
        Self {
            base: BaseCleaner::new(trash_mover, audit_logger),
            cache_path: home.join(".npm/_cacache"),
        }
    }
}

impl Cleaner for NpmCleaner {
    fn id(&self) -> &'static str {
        "npm"
    }

    fn name(&self) -> &'static str {
        "npm Cache"
    }

    fn description(&self) -> &'static str {
        "npm package cache (~/.npm/_cacache)"
    }

    fn category(&self) -> CleanerCategory {
        CleanerCategory::Developer
    }

    fn safe_boundary(&self) -> SafeBoundary {
        presets::npm()
    }

    fn is_available(&self) -> bool {
        self.cache_path.exists()
    }

    fn scan(&self, ctx: &CleanerContext) -> ScanResult {
        if !self.cache_path.exists() {
            return ScanResult::empty(self.id());
        }

        // For npm, we clean the entire _cacache directory
        let options = ScanOptions {
            top_level_only: true,
            directories_only: true,
            ..Default::default()
        };

        self.base.scan_dirs(
            self.id(),
            &[self.cache_path.clone()],
            &self.safe_boundary(),
            &options,
            ctx,
        )
    }

    fn clean(&self, items: &[CleanableItem], ctx: &CleanerContext) -> CleanResult {
        self.base
            .clean_items(self.id(), items, &self.safe_boundary(), ctx)
    }
}
