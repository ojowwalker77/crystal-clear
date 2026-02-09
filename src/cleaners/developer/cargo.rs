//! Cargo cache cleaner.

use std::path::PathBuf;

use crate::audit::AuditLogger;
use crate::cleaners::base::BaseCleaner;
use crate::cleaners::traits::*;
use crate::config::schema::CargoConfig;
use crate::safety::boundaries::presets;
use crate::safety::SafeBoundary;
use crate::scanner::ScanOptions;
use crate::trash::TrashMover;

/// Cleaner for Cargo (Rust) cache.
pub struct CargoCleaner {
    base: BaseCleaner,
    cache_paths: Vec<PathBuf>,
    keep_recent_days: u32,
}

impl CargoCleaner {
    pub fn new(trash_mover: TrashMover, audit_logger: AuditLogger, config: &CargoConfig) -> Self {
        let home = dirs::home_dir().expect("Home directory required");
        let cargo = home.join(".cargo");

        Self {
            base: BaseCleaner::new(trash_mover, audit_logger),
            cache_paths: vec![cargo.join("registry/cache"), cargo.join("registry/src")],
            keep_recent_days: config.keep_recent_days,
        }
    }
}

impl Cleaner for CargoCleaner {
    fn id(&self) -> &'static str {
        "cargo"
    }

    fn name(&self) -> &'static str {
        "Cargo Cache"
    }

    fn description(&self) -> &'static str {
        "Rust/Cargo package cache (~/.cargo/registry)"
    }

    fn category(&self) -> CleanerCategory {
        CleanerCategory::Developer
    }

    fn safe_boundary(&self) -> SafeBoundary {
        presets::cargo()
    }

    fn is_available(&self) -> bool {
        self.cache_paths.iter().any(|p| p.exists())
    }

    fn scan(&self, ctx: &CleanerContext) -> ScanResult {
        let options = ScanOptions {
            top_level_only: true,
            directories_only: true,
            min_age_days: if self.keep_recent_days > 0 {
                Some(self.keep_recent_days)
            } else {
                None
            },
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
        self.base
            .clean_items(self.id(), items, &self.safe_boundary(), ctx)
    }
}
