//! Application leftovers cleaner.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

use crate::audit::AuditLogger;
use crate::cleaners::traits::*;
use crate::config::schema::AppsConfig;
use crate::safety::SafeBoundary;
use crate::scanner::{scan_directory, ScanOptions};

/// Cleaner for leftover files from uninstalled applications.
pub struct AppsLeftoversCleaner {
    audit_logger: AuditLogger,
    known_apps: HashSet<String>,
    search_paths: Vec<PathBuf>,
    home_dir: PathBuf,
}

impl AppsLeftoversCleaner {
    pub fn new(audit_logger: AuditLogger, config: &AppsConfig) -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));

        Self {
            audit_logger,
            known_apps: config.known_apps.iter().cloned().collect(),
            search_paths: vec![
                home.join("Library/Application Support"),
                home.join("Library/Preferences"),
                home.join("Library/Caches"),
                home.join("Library/Logs"),
                home.join("Library/Containers"),
                home.join("Library/Group Containers"),
                home.join("Library/Saved Application State"),
            ],
            home_dir: home,
        }
    }

    fn get_installed_apps() -> HashSet<String> {
        let mut apps = HashSet::new();

        // Scan /Applications
        if let Ok(entries) = std::fs::read_dir("/Applications") {
            for entry in entries.filter_map(|e| e.ok()) {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".app") {
                        // Extract bundle ID if possible
                        let info_plist = entry.path().join("Contents/Info.plist");
                        if let Some(bundle_id) = Self::get_bundle_id(&info_plist) {
                            apps.insert(bundle_id);
                        }
                        // Also add the app name without .app
                        apps.insert(name.trim_end_matches(".app").to_string());
                    }
                }
            }
        }

        // Scan ~/Applications
        if let Some(home) = dirs::home_dir() {
            if let Ok(entries) = std::fs::read_dir(home.join("Applications")) {
                for entry in entries.filter_map(|e| e.ok()) {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.ends_with(".app") {
                            let info_plist = entry.path().join("Contents/Info.plist");
                            if let Some(bundle_id) = Self::get_bundle_id(&info_plist) {
                                apps.insert(bundle_id);
                            }
                            apps.insert(name.trim_end_matches(".app").to_string());
                        }
                    }
                }
            }
        }

        apps
    }

    fn get_bundle_id(plist_path: &PathBuf) -> Option<String> {
        // Simple plist parsing - just look for CFBundleIdentifier
        let content = std::fs::read_to_string(plist_path).ok()?;

        // Find CFBundleIdentifier and get the next string value
        let key_pos = content.find("<key>CFBundleIdentifier</key>")?;
        let after_key = &content[key_pos..];
        let string_start = after_key.find("<string>")? + 8;
        let string_end = after_key[string_start..].find("</string>")?;

        Some(after_key[string_start..string_start + string_end].to_string())
    }

    fn is_leftover(&self, name: &str, installed: &HashSet<String>) -> bool {
        // Check if explicitly listed as known uninstalled app
        if self.known_apps.contains(name) {
            return true;
        }

        // Use stricter matching to avoid false positives
        // Only match if:
        // 1. Exact match on bundle ID
        // 2. Name matches exactly (case-insensitive)
        // 3. Name starts with bundle ID prefix for that app
        let name_lower = name.to_lowercase();

        for app in installed {
            let app_lower = app.to_lowercase();

            // Exact match
            if name_lower == app_lower {
                return false;
            }

            // Bundle ID style match (e.g., "com.apple.Safari" matches "Safari")
            if name.starts_with("com.") || name.starts_with("org.") || name.starts_with("io.") {
                if app == name {
                    return false;
                }
                // Check if this bundle ID belongs to an installed app
                // e.g., com.apple.Safari -> Safari
                if let Some(last_part) = name.rsplit('.').next() {
                    if last_part.to_lowercase() == app_lower {
                        return false;
                    }
                }
            }

            // Only do contains check for longer app names (5+ chars) to avoid
            // false matches on short names like "Go", "Pro", "Code"
            if app.len() >= 5 && name_lower.contains(&app_lower) {
                return false;
            }
        }

        false // By default, assume it's not a leftover (safe approach)
    }
}

impl Cleaner for AppsLeftoversCleaner {
    fn id(&self) -> &'static str {
        "apps"
    }

    fn name(&self) -> &'static str {
        "App Leftovers"
    }

    fn description(&self) -> &'static str {
        "Leftover files from uninstalled applications"
    }

    fn category(&self) -> CleanerCategory {
        CleanerCategory::Apps
    }

    fn safe_boundary(&self) -> SafeBoundary {
        SafeBoundary::new(self.search_paths.clone(), "Application support directories")
    }

    fn is_available(&self) -> bool {
        !self.known_apps.is_empty()
    }

    fn scan(&self, _ctx: &CleanerContext) -> ScanResult {
        let start = Instant::now();
        let installed = Self::get_installed_apps();
        let mut items = Vec::new();
        let errors = Vec::new();

        for search_path in &self.search_paths {
            if !search_path.exists() {
                continue;
            }

            let boundary = SafeBoundary::new(
                vec![search_path.clone()],
                "App search directory",
            );

            let options = ScanOptions {
                top_level_only: true,
                directories_only: true,
                ..Default::default()
            };

            let scanned = scan_directory(search_path, &boundary, &options);

            for item in scanned {
                let name = item
                    .path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                if self.is_leftover(&name, &installed) {
                    items.push(CleanableItem {
                        path: item.path,
                        size: item.size,
                        item_type: ItemType::Directory,
                        age_days: item.age_days,
                        description: format!("Leftover: {}", name),
                        requires_force: true, // Leftovers require force
                        risk_level: RiskLevel::High,
                    });
                }
            }
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
        let start = Instant::now();
        let mut cleaned = 0;
        let mut bytes_freed = 0u64;
        let mut failed = Vec::new();

        for item in items {
            if !ctx.force {
                failed.push((
                    item.path.clone(),
                    "App leftovers require --force flag".to_string(),
                ));
                continue;
            }

            if ctx.dry_run {
                let _ = self.audit_logger.log_dry_run(self.id(), &item.path, item.size);
                cleaned += 1;
                bytes_freed += item.size;
                continue;
            }

            // Move to trash (using stored home_dir to avoid repeat lookups)
            let trash = self.home_dir.join(".Trash");
            let trash_name = format!(
                "{}_{}",
                item.path.file_name().unwrap_or_default().to_string_lossy(),
                chrono::Utc::now().format("%Y%m%d_%H%M%S")
            );
            let trash_dest = trash.join(trash_name);

            match std::fs::rename(&item.path, &trash_dest) {
                Ok(()) => {
                    let _ = self.audit_logger.log_cleaned(self.id(), &item.path, item.size);
                    cleaned += 1;
                    bytes_freed += item.size;
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    let _ = self.audit_logger.log_error(self.id(), &item.path, &error_msg);
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
