//! System logs cleaner.

use std::path::PathBuf;

use crate::audit::AuditLogger;
use crate::cleaners::base::BaseCleaner;
use crate::cleaners::traits::*;
use crate::config::schema::CategoryConfig;
use crate::safety::boundaries::presets;
use crate::safety::SafeBoundary;
use crate::scanner::ScanOptions;
use crate::trash::TrashMover;

/// Cleaner for user log directories.
pub struct SystemLogsCleaner {
    base: BaseCleaner,
    paths: Vec<PathBuf>,
    exclude: Vec<PathBuf>,
}

impl SystemLogsCleaner {
    pub fn new(
        trash_mover: TrashMover,
        audit_logger: AuditLogger,
        config: &CategoryConfig,
    ) -> Self {
        let home = dirs::home_dir().expect("Home directory required");
        let mut paths = vec![home.join("Library/Logs")];

        paths.extend(config.paths.iter().cloned());

        Self {
            base: BaseCleaner::new(trash_mover, audit_logger),
            paths,
            exclude: config.exclude.clone(),
        }
    }
}

impl Cleaner for SystemLogsCleaner {
    fn id(&self) -> &'static str {
        "system-logs"
    }

    fn name(&self) -> &'static str {
        "System Logs"
    }

    fn description(&self) -> &'static str {
        "User application logs (~/Library/Logs)"
    }

    fn category(&self) -> CleanerCategory {
        CleanerCategory::System
    }

    fn safe_boundary(&self) -> SafeBoundary {
        presets::user_logs()
    }

    fn is_available(&self) -> bool {
        self.paths.iter().any(|p| p.exists())
    }

    fn scan(&self, ctx: &CleanerContext) -> ScanResult {
        let options = ScanOptions {
            top_level_only: true,
            directories_only: true,
            ..Default::default()
        };

        let mut result =
            self.base
                .scan_dirs(self.id(), &self.paths, &self.safe_boundary(), &options, ctx);

        result.items.retain(|item| {
            !self
                .exclude
                .iter()
                .any(|excl| item.path.starts_with(excl) || item.path == *excl)
        });

        result.total_size = result.items.iter().map(|i| i.size).sum();

        result
    }

    fn clean(&self, items: &[CleanableItem], ctx: &CleanerContext) -> CleanResult {
        self.base
            .clean_items(self.id(), items, &self.safe_boundary(), ctx)
    }
}
