//! pip cache cleaner.

use std::path::PathBuf;

use crate::audit::AuditLogger;
use crate::cleaners::base::BaseCleaner;
use crate::cleaners::traits::*;
use crate::config::schema::CategoryConfig;
use crate::safety::boundaries::presets;
use crate::safety::SafeBoundary;
use crate::scanner::ScanOptions;
use crate::trash::TrashMover;

/// Cleaner for pip (Python) cache.
pub struct PipCleaner {
    base: BaseCleaner,
    cache_paths: Vec<PathBuf>,
}

impl PipCleaner {
    pub fn new(
        trash_mover: TrashMover,
        audit_logger: AuditLogger,
        _config: &CategoryConfig,
    ) -> Self {
        let home = dirs::home_dir().expect("Home directory required");

        Self {
            base: BaseCleaner::new(trash_mover, audit_logger),
            cache_paths: vec![
                home.join("Library/Caches/pip"),
                home.join(".cache/pip"),
            ],
        }
    }
}

impl Cleaner for PipCleaner {
    fn id(&self) -> &'static str {
        "pip"
    }

    fn name(&self) -> &'static str {
        "pip Cache"
    }

    fn description(&self) -> &'static str {
        "Python pip package cache"
    }

    fn category(&self) -> CleanerCategory {
        CleanerCategory::Developer
    }

    fn safe_boundary(&self) -> SafeBoundary {
        presets::pip()
    }

    fn is_available(&self) -> bool {
        self.cache_paths.iter().any(|p| p.exists())
    }

    fn scan(&self, ctx: &CleanerContext) -> ScanResult {
        let options = ScanOptions {
            top_level_only: true,
            directories_only: true,
            ..Default::default()
        };

        self.base.scan_dirs(
            self.id(),
            &self.cache_paths,
            &self.safe_boundary(),
            &options,
            ctx,
        )
    }

    fn clean(&self, items: &[CleanableItem], ctx: &CleanerContext) -> CleanResult {
        self.base.clean_items(self.id(), items, &self.safe_boundary(), ctx)
    }
}
