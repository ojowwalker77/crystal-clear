//! Dynamic cleaner that uses discovered paths.

use std::time::Instant;

use crate::audit::AuditLogger;
use crate::cleaners::base::BaseCleaner;
use crate::cleaners::traits::*;
use crate::db::{Database, PathCategory};
use crate::discovery::heuristics;
use crate::safety::SafeBoundary;
use crate::scanner::calculate_size;
use crate::trash::TrashMover;

/// Cleaner that operates on dynamically discovered paths.
pub struct DynamicCleaner {
    base: BaseCleaner,
    db: Database,
    category: PathCategory,
}

impl DynamicCleaner {
    /// Create a new dynamic cleaner for a specific category.
    pub fn new(
        trash_mover: TrashMover,
        audit_logger: AuditLogger,
        db: Database,
        category: PathCategory,
    ) -> Self {
        Self {
            base: BaseCleaner::new(trash_mover, audit_logger),
            db,
            category,
        }
    }

    /// Get the category this cleaner operates on.
    pub fn category(&self) -> &PathCategory {
        &self.category
    }
}

impl Cleaner for DynamicCleaner {
    fn id(&self) -> &'static str {
        match self.category {
            PathCategory::Cache => "dynamic-cache",
            PathCategory::Logs => "dynamic-logs",
            PathCategory::Temp => "dynamic-temp",
            PathCategory::Build => "dynamic-build",
            PathCategory::Deps => "dynamic-deps",
            PathCategory::Archive => "dynamic-archive",
            PathCategory::Other => "dynamic-other",
        }
    }

    fn name(&self) -> &'static str {
        // This is a bit of a hack to return a static str
        // In practice, we'll use the instance name field
        match self.category {
            PathCategory::Cache => "Discovered Caches",
            PathCategory::Logs => "Discovered Logs",
            PathCategory::Temp => "Discovered Temp",
            PathCategory::Build => "Discovered Build",
            PathCategory::Deps => "Discovered Dependencies",
            PathCategory::Archive => "Discovered Archives",
            PathCategory::Other => "Discovered Other",
        }
    }

    fn description(&self) -> &'static str {
        match self.category {
            PathCategory::Cache => "Cache directories found by path discovery",
            PathCategory::Logs => "Log directories found by path discovery",
            PathCategory::Temp => "Temporary directories found by path discovery",
            PathCategory::Build => "Build artifact directories found by path discovery",
            PathCategory::Deps => "Dependency directories found by path discovery",
            PathCategory::Archive => "Archive directories found by path discovery",
            PathCategory::Other => "Other directories found by path discovery",
        }
    }

    fn category(&self) -> CleanerCategory {
        CleanerCategory::System
    }

    fn safe_boundary(&self) -> SafeBoundary {
        // Get all discovered paths for this category and create boundary
        let paths = self
            .db
            .get_paths_by_category(&self.category)
            .unwrap_or_default()
            .into_iter()
            .map(|p| p.path)
            .collect();

        SafeBoundary::new(
            paths,
            format!("Discovered {} paths", self.category.as_str()),
        )
    }

    fn is_available(&self) -> bool {
        self.db
            .get_paths_by_category(&self.category)
            .map(|paths| !paths.is_empty())
            .unwrap_or(false)
    }

    fn scan(&self, ctx: &CleanerContext) -> ScanResult {
        let start = Instant::now();
        let mut items = Vec::new();
        let errors = Vec::new();

        let paths = match self.db.get_paths_by_category(&self.category) {
            Ok(p) => p,
            Err(e) => {
                return ScanResult {
                    category: self.id().to_string(),
                    items,
                    total_size: 0,
                    scan_duration: start.elapsed(),
                    errors: vec![e.to_string()],
                };
            }
        };

        for discovered in paths {
            if !discovered.path.exists() {
                continue;
            }

            let size = discovered
                .last_size
                .unwrap_or_else(|| calculate_size(&discovered.path));

            // Apply age filter
            if ctx.keep_recent_days > 0 {
                if let Ok(metadata) = discovered.path.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        let age_days = modified
                            .elapsed()
                            .map(|d| (d.as_secs() / 86400) as u32)
                            .unwrap_or(0);
                        if age_days < ctx.keep_recent_days {
                            continue;
                        }
                    }
                }
            }

            // Determine risk level based on confidence
            let risk_level = if discovered.confidence >= 0.9 {
                RiskLevel::Low
            } else if discovered.confidence >= 0.6 {
                RiskLevel::Medium
            } else {
                RiskLevel::High
            };

            // Calculate risk score from heuristics
            let _risk_score = heuristics::calculate_risk_score(&discovered.path, &self.category);

            items.push(CleanableItem {
                path: discovered.path.clone(),
                size,
                item_type: ItemType::Directory,
                age_days: None,
                description: format!(
                    "{} ({:.0}% confidence)",
                    discovered
                        .path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy(),
                    discovered.confidence * 100.0
                ),
                requires_force: discovered.confidence < 0.7,
                risk_level,
            });
        }

        let total_size = items.iter().map(|i| i.size).sum();

        ScanResult {
            category: self.id().to_string(),
            items,
            total_size,
            scan_duration: start.elapsed(),
            errors,
        }
    }

    fn clean(&self, items: &[CleanableItem], ctx: &CleanerContext) -> CleanResult {
        let result = self
            .base
            .clean_items(self.id(), items, &self.safe_boundary(), ctx);

        // Record cleaned paths
        for item in items {
            if let Err(e) = self.db.record_path_cleaned(&item.path) {
                tracing::warn!("Failed to record cleaned path: {}", e);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_dynamic_cleaner_creation() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.db");
        let db = Database::open_at(db_path).unwrap();

        let trash_mover = TrashMover::default_mover().unwrap();
        let audit_logger = AuditLogger::default_logger(false);

        let cleaner = DynamicCleaner::new(trash_mover, audit_logger, db, PathCategory::Cache);

        assert_eq!(cleaner.id(), "dynamic-cache");
        assert!(!cleaner.is_available()); // No paths discovered yet
    }
}
