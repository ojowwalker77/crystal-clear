//! Xcode cleaner - macOS only.
//!
//! This cleaner is only available on macOS as Xcode is a macOS-exclusive tool.

#![cfg(target_os = "macos")]

use std::path::PathBuf;
use std::time::Instant;

use crate::audit::AuditLogger;
use crate::cleaners::base::BaseCleaner;
use crate::cleaners::traits::*;
use crate::config::schema::XcodeConfig;
use crate::safety::boundaries::presets;
use crate::safety::SafeBoundary;
use crate::scanner::ScanOptions;
use crate::trash::TrashMover;

/// Cleaner for Xcode developer directories.
pub struct XcodeCleaner {
    base: BaseCleaner,
    derived_data_path: PathBuf,
    archives_path: PathBuf,
    device_support_paths: Vec<PathBuf>,
    keep_recent_archives_days: u32,
    keep_ios_versions: Vec<String>,
}

impl XcodeCleaner {
    pub fn new(
        trash_mover: TrashMover,
        audit_logger: AuditLogger,
        config: &XcodeConfig,
    ) -> Self {
        let home = dirs::home_dir().expect("Home directory required");
        let developer = home.join("Library/Developer/Xcode");

        Self {
            base: BaseCleaner::new(trash_mover, audit_logger),
            derived_data_path: developer.join("DerivedData"),
            archives_path: developer.join("Archives"),
            device_support_paths: vec![
                developer.join("iOS DeviceSupport"),
                developer.join("watchOS DeviceSupport"),
                developer.join("tvOS DeviceSupport"),
            ],
            keep_recent_archives_days: config.keep_recent_archives_days,
            keep_ios_versions: config.keep_ios_versions.clone(),
        }
    }

    fn scan_derived_data(&self, ctx: &CleanerContext) -> Vec<CleanableItem> {
        if !self.derived_data_path.exists() {
            return Vec::new();
        }

        let boundary = SafeBoundary::new(
            vec![self.derived_data_path.clone()],
            "Xcode DerivedData",
        );

        let options = ScanOptions {
            top_level_only: true,
            directories_only: true,
            ..Default::default()
        };

        let result = self.base.scan_dirs(
            "xcode-deriveddata",
            &[self.derived_data_path.clone()],
            &boundary,
            &options,
            ctx,
        );

        result.items
    }

    fn scan_archives(&self, ctx: &CleanerContext) -> Vec<CleanableItem> {
        if !self.archives_path.exists() {
            return Vec::new();
        }

        let boundary = SafeBoundary::new(
            vec![self.archives_path.clone()],
            "Xcode Archives",
        );

        let options = ScanOptions {
            directories_only: true,
            min_age_days: if self.keep_recent_archives_days > 0 {
                Some(self.keep_recent_archives_days)
            } else {
                None
            },
            ..Default::default()
        };

        let result = self.base.scan_dirs(
            "xcode-archives",
            &[self.archives_path.clone()],
            &boundary,
            &options,
            ctx,
        );

        // Filter to only .xcarchive bundles and mark as medium risk
        result
            .items
            .into_iter()
            .filter(|item| {
                item.path
                    .extension()
                    .map(|e| e == "xcarchive")
                    .unwrap_or(false)
            })
            .map(|mut item| {
                item.risk_level = RiskLevel::Medium;
                item.description = format!("Archive: {}", item.description);
                item
            })
            .collect()
    }

    fn scan_device_support(&self, ctx: &CleanerContext) -> Vec<CleanableItem> {
        let mut items = Vec::new();

        for path in &self.device_support_paths {
            if !path.exists() {
                continue;
            }

            let boundary = SafeBoundary::new(vec![path.clone()], "Device Support");

            let options = ScanOptions {
                top_level_only: true,
                directories_only: true,
                ..Default::default()
            };

            let result = self.base.scan_dirs(
                "xcode-devicesupport",
                &[path.clone()],
                &boundary,
                &options,
                ctx,
            );

            for mut item in result.items {
                // Check if this version should be kept
                let should_keep = self.keep_ios_versions.iter().any(|v| {
                    item.path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|name| name.contains(v))
                        .unwrap_or(false)
                });

                if !should_keep {
                    item.description = format!(
                        "Device Support: {}",
                        item.path.file_name().unwrap_or_default().to_string_lossy()
                    );
                    items.push(item);
                }
            }
        }

        items
    }
}

impl Cleaner for XcodeCleaner {
    fn id(&self) -> &'static str {
        "xcode"
    }

    fn name(&self) -> &'static str {
        "Xcode"
    }

    fn description(&self) -> &'static str {
        "Xcode build artifacts (DerivedData, Archives, DeviceSupport)"
    }

    fn category(&self) -> CleanerCategory {
        CleanerCategory::Developer
    }

    fn safe_boundary(&self) -> SafeBoundary {
        presets::xcode()
    }

    fn is_available(&self) -> bool {
        std::path::Path::new("/Applications/Xcode.app").exists()
            || self.derived_data_path.exists()
    }

    fn scan(&self, ctx: &CleanerContext) -> ScanResult {
        let start = Instant::now();
        let mut items = Vec::new();
        let errors = Vec::new();

        items.extend(self.scan_derived_data(ctx));
        items.extend(self.scan_archives(ctx));
        items.extend(self.scan_device_support(ctx));

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
        self.base.clean_items(self.id(), items, &self.safe_boundary(), ctx)
    }
}
