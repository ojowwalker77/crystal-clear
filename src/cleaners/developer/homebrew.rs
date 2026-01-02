//! Homebrew cleaner - macOS and Linux only.
//!
//! This cleaner supports both macOS and Linux (via Linuxbrew).

#![cfg(any(target_os = "macos", target_os = "linux"))]

use std::process::Command;
use std::time::Instant;

use crate::audit::AuditLogger;
use crate::cleaners::traits::*;
use crate::config::schema::HomebrewConfig;
use crate::safety::boundaries::presets;
use crate::safety::SafeBoundary;

/// Cleaner for Homebrew cache and old versions.
#[allow(dead_code)]
pub struct HomebrewCleaner {
    audit_logger: AuditLogger,
    keep_versions: u32,
}

impl HomebrewCleaner {
    pub fn new(audit_logger: AuditLogger, config: &HomebrewConfig) -> Self {
        Self {
            audit_logger,
            keep_versions: config.keep_versions,
        }
    }

    fn get_brew_path() -> Option<std::path::PathBuf> {
        // Check common Homebrew locations
        #[cfg(target_os = "macos")]
        let paths = [
            "/opt/homebrew/bin/brew",       // Apple Silicon
            "/usr/local/bin/brew",          // Intel Mac
        ];

        #[cfg(target_os = "linux")]
        let paths = [
            "/home/linuxbrew/.linuxbrew/bin/brew",  // Linuxbrew system install
            "/usr/local/bin/brew",                   // Standard location
        ];

        for path in paths {
            let p = std::path::PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }

        // Try to find in PATH
        Command::new("which")
            .arg("brew")
            .output()
            .ok()
            .and_then(|out| {
                String::from_utf8(out.stdout)
                    .ok()
                    .map(|s| std::path::PathBuf::from(s.trim()))
            })
    }

    fn estimate_cleanup_size(&self) -> Option<u64> {
        let brew = Self::get_brew_path()?;

        // Run brew cleanup --dry-run to estimate
        let output = Command::new(&brew)
            .args(["cleanup", "--dry-run"])
            .output()
            .ok()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Count lines and estimate ~50MB per package (rough estimate)
            let count = stdout.lines().count();
            Some(count as u64 * 50 * 1024 * 1024)
        } else {
            None
        }
    }
}

impl Cleaner for HomebrewCleaner {
    fn id(&self) -> &'static str {
        "homebrew"
    }

    fn name(&self) -> &'static str {
        "Homebrew"
    }

    fn description(&self) -> &'static str {
        "Homebrew cache and old package versions"
    }

    fn category(&self) -> CleanerCategory {
        CleanerCategory::Developer
    }

    fn safe_boundary(&self) -> SafeBoundary {
        presets::homebrew()
    }

    fn is_available(&self) -> bool {
        Self::get_brew_path().is_some()
    }

    fn scan(&self, _ctx: &CleanerContext) -> ScanResult {
        let start = Instant::now();

        let brew = match Self::get_brew_path() {
            Some(b) => b,
            None => return ScanResult::empty(self.id()),
        };

        // Run brew cleanup --dry-run to see what would be cleaned
        let output = match Command::new(&brew)
            .args(["cleanup", "--dry-run"])
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                return ScanResult {
                    category: self.id().to_string(),
                    items: Vec::new(),
                    total_size: 0,
                    scan_duration: start.elapsed(),
                    errors: vec![format!("Failed to run brew cleanup --dry-run: {}", e)],
                };
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let items: Vec<CleanableItem> = stdout
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with("==>"))
            .map(|line| {
                let path = std::path::PathBuf::from(line.trim());
                CleanableItem {
                    size: 0, // Homebrew doesn't report sizes in dry-run
                    path,
                    item_type: ItemType::Directory,
                    age_days: None,
                    description: line.to_string(),
                    requires_force: false,
                    risk_level: RiskLevel::Low,
                }
            })
            .collect();

        let estimated_size = self.estimate_cleanup_size().unwrap_or(0);

        ScanResult {
            category: self.id().to_string(),
            items,
            total_size: estimated_size,
            scan_duration: start.elapsed(),
            errors: Vec::new(),
        }
    }

    fn clean(&self, _items: &[CleanableItem], ctx: &CleanerContext) -> CleanResult {
        let start = Instant::now();

        if ctx.dry_run {
            // In dry-run, just report what would be cleaned
            let scan = self.scan(ctx);
            // Log dry-run for each item
            for item in &scan.items {
                let _ = self.audit_logger.log_dry_run(self.id(), &item.path, item.size);
            }
            return CleanResult {
                category: self.id().to_string(),
                items_cleaned: scan.items.len(),
                bytes_freed: scan.total_size,
                items_failed: Vec::new(),
                clean_duration: start.elapsed(),
            };
        }

        let brew = match Self::get_brew_path() {
            Some(b) => b,
            None => return CleanResult::empty(self.id()),
        };

        // Run actual cleanup
        let output = Command::new(&brew).args(["cleanup"]).output();

        match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let cleaned_count = stdout.lines().count();

                // Log each cleaned item
                for line in stdout.lines() {
                    if !line.is_empty() && !line.starts_with("==>") {
                        let path = std::path::PathBuf::from(line.trim());
                        let _ = self.audit_logger.log_cleaned(self.id(), &path, 0);
                    }
                }

                CleanResult {
                    category: self.id().to_string(),
                    items_cleaned: cleaned_count,
                    bytes_freed: 0, // Homebrew doesn't report this
                    items_failed: Vec::new(),
                    clean_duration: start.elapsed(),
                }
            }
            Ok(o) => {
                let error_msg = String::from_utf8_lossy(&o.stderr).to_string();
                let path = std::path::PathBuf::from("brew cleanup");
                let _ = self.audit_logger.log_error(self.id(), &path, &error_msg);
                CleanResult {
                    category: self.id().to_string(),
                    items_cleaned: 0,
                    bytes_freed: 0,
                    items_failed: vec![(path, error_msg)],
                    clean_duration: start.elapsed(),
                }
            },
            Err(e) => {
                let path = std::path::PathBuf::from("brew cleanup");
                let _ = self.audit_logger.log_error(self.id(), &path, &e.to_string());
                CleanResult {
                    category: self.id().to_string(),
                    items_cleaned: 0,
                    bytes_freed: 0,
                    items_failed: vec![(path, e.to_string())],
                    clean_duration: start.elapsed(),
                }
            },
        }
    }

    fn estimate_size(&self) -> Option<u64> {
        self.estimate_cleanup_size()
    }
}
