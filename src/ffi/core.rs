//! Core FFI functions exposed to Swift.
//!
//! These functions wrap the internal Rust functionality and handle
//! type conversion to/from FFI-safe types.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::audit::AuditLogger;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use crate::cleaners::HomebrewCleaner;
#[cfg(target_os = "macos")]
use crate::cleaners::XcodeCleaner;
use crate::cleaners::{
    AppsLeftoversCleaner, CargoCleaner, CleanableItem, Cleaner, CleanerContext, DockerCleaner,
    NpmCleaner, PipCleaner, SystemCachesCleaner, SystemLogsCleaner, TrashCleaner, YarnCleaner,
};
use crate::disk;
use crate::safety::protected_paths;
use crate::scanner::apps;
use crate::trash::TrashMover;

use super::types::{
    FFICategorySize, FFICleanFailure, FFICleanOptions, FFICleanResult, FFICleanableItem,
    FFICleanerInfo, FFIDiskInfo, FFIDownloadCategory, FFIDownloadsScanOptions,
    FFIDownloadsScanResult, FFIDuplicateGroup, FFIDuplicateScanOptions, FFIDuplicateScanResult,
    FFIInstalledApp, FFILargeFileScanOptions, FFIScanOptions, FFIScanResult,
};

fn runtime_config() -> crate::config::Config {
    crate::config::load_config(None).unwrap_or_default()
}

/// Get all available cleaners with default configuration.
fn get_cleaners_with_config(config: &crate::config::Config) -> Vec<Arc<dyn Cleaner>> {
    let mut cleaners: Vec<Arc<dyn Cleaner>> = Vec::new();

    // Cleaners that need TrashMover
    if config.categories.system_cache.enabled {
        if let Ok(tm) = TrashMover::default_mover() {
            cleaners.push(Arc::new(SystemCachesCleaner::new(
                tm,
                AuditLogger::default_logger(config.safety.audit_enabled),
                &config.categories.system_cache,
            )));
        }
    }

    if config.categories.system_logs.enabled {
        if let Ok(tm) = TrashMover::default_mover() {
            cleaners.push(Arc::new(SystemLogsCleaner::new(
                tm,
                AuditLogger::default_logger(config.safety.audit_enabled),
                &config.categories.system_logs,
            )));
        }
    }

    if config.categories.npm.enabled {
        if let Ok(tm) = TrashMover::default_mover() {
            cleaners.push(Arc::new(NpmCleaner::new(
                tm,
                AuditLogger::default_logger(config.safety.audit_enabled),
                &config.categories.npm,
            )));
        }
    }

    if config.categories.yarn.enabled {
        if let Ok(tm) = TrashMover::default_mover() {
            cleaners.push(Arc::new(YarnCleaner::new(
                tm,
                AuditLogger::default_logger(config.safety.audit_enabled),
                &config.categories.yarn,
            )));
        }
    }

    if config.categories.cargo.base.enabled {
        if let Ok(tm) = TrashMover::default_mover() {
            cleaners.push(Arc::new(CargoCleaner::new(
                tm,
                AuditLogger::default_logger(config.safety.audit_enabled),
                &config.categories.cargo,
            )));
        }
    }

    if config.categories.pip.enabled {
        if let Ok(tm) = TrashMover::default_mover() {
            cleaners.push(Arc::new(PipCleaner::new(
                tm,
                AuditLogger::default_logger(config.safety.audit_enabled),
                &config.categories.pip,
            )));
        }
    }

    // Cleaners without TrashMover
    if config.categories.trash.base.enabled {
        cleaners.push(Arc::new(TrashCleaner::new(
            AuditLogger::default_logger(config.safety.audit_enabled),
            &config.categories.trash,
        )));
    }

    if config.categories.docker.base.enabled {
        cleaners.push(Arc::new(DockerCleaner::new(
            AuditLogger::default_logger(config.safety.audit_enabled),
            &config.categories.docker,
        )));
    }

    if config.categories.apps.base.enabled {
        cleaners.push(Arc::new(AppsLeftoversCleaner::new(
            AuditLogger::default_logger(config.safety.audit_enabled),
            &config.categories.apps,
        )));
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    if config.categories.homebrew.base.enabled {
        cleaners.push(Arc::new(HomebrewCleaner::new(
            AuditLogger::default_logger(config.safety.audit_enabled),
            &config.categories.homebrew,
        )));
    }

    #[cfg(target_os = "macos")]
    if config.categories.xcode.base.enabled {
        if let Ok(tm) = TrashMover::default_mover() {
            cleaners.push(Arc::new(XcodeCleaner::new(
                tm,
                AuditLogger::default_logger(config.safety.audit_enabled),
                &config.categories.xcode,
            )));
        }
    }

    cleaners
}

fn get_cleaners() -> Vec<Arc<dyn Cleaner>> {
    let config = runtime_config();
    get_cleaners_with_config(&config)
}

fn path_size(path: &PathBuf) -> u64 {
    if path.is_dir() {
        walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum()
    } else {
        std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
    }
}

fn build_scan_index(
    cleaners: &[Arc<dyn Cleaner>],
    keep_recent_days: u32,
) -> HashMap<String, (usize, CleanableItem)> {
    let ctx = CleanerContext {
        dry_run: true,
        force: true,
        keep_recent_days,
        min_size: None,
        max_items: None,
        skip_confirm: true,
    };

    let mut index = HashMap::new();
    for (idx, cleaner) in cleaners.iter().enumerate() {
        if !cleaner.is_available() {
            continue;
        }
        let result = cleaner.scan(&ctx);
        for item in result.items {
            index.insert(item.path.to_string_lossy().to_string(), (idx, item));
        }
    }

    index
}

/// Get information about all available cleaners.
#[uniffi::export]
pub fn get_available_cleaners() -> Vec<FFICleanerInfo> {
    get_cleaners()
        .into_iter()
        .map(|c| FFICleanerInfo {
            id: c.id().to_string(),
            name: c.name().to_string(),
            description: c.description().to_string(),
            category: c.category().into(),
            available: c.is_available(),
        })
        .collect()
}

/// Scan all categories for cleanable items.
#[uniffi::export]
pub fn scan_all_categories(options: FFIScanOptions) -> Vec<FFIScanResult> {
    let ctx = CleanerContext {
        dry_run: options.dry_run,
        force: false,
        keep_recent_days: options.keep_recent_days,
        min_size: options.min_size,
        max_items: options.max_items.map(|n| n as usize),
        skip_confirm: true,
    };

    get_cleaners()
        .into_iter()
        .filter(|c| c.is_available())
        .map(|c| {
            let result = c.scan(&ctx);
            FFIScanResult {
                category: c.id().to_string(),
                category_name: c.name().to_string(),
                items: result.items.into_iter().map(|i| i.into()).collect(),
                total_size: result.total_size,
                scan_duration_ms: result.scan_duration.as_millis() as u64,
                errors: result.errors,
            }
        })
        .collect()
}

/// Scan a specific category for cleanable items.
#[uniffi::export]
pub fn scan_category(category_id: String, options: FFIScanOptions) -> FFIScanResult {
    let ctx = CleanerContext {
        dry_run: options.dry_run,
        force: false,
        keep_recent_days: options.keep_recent_days,
        min_size: options.min_size,
        max_items: options.max_items.map(|n| n as usize),
        skip_confirm: true,
    };

    let cleaner = get_cleaners().into_iter().find(|c| c.id() == category_id);

    match cleaner {
        Some(c) if c.is_available() => {
            let result = c.scan(&ctx);
            FFIScanResult {
                category: c.id().to_string(),
                category_name: c.name().to_string(),
                items: result.items.into_iter().map(|i| i.into()).collect(),
                total_size: result.total_size,
                scan_duration_ms: result.scan_duration.as_millis() as u64,
                errors: result.errors,
            }
        }
        _ => FFIScanResult {
            category: category_id,
            category_name: "Unknown".to_string(),
            items: vec![],
            total_size: 0,
            scan_duration_ms: 0,
            errors: vec!["Category not found or not available".to_string()],
        },
    }
}

/// Scan for installed applications.
#[uniffi::export]
pub fn scan_applications() -> Vec<FFIInstalledApp> {
    apps::scan_applications()
        .into_iter()
        .map(|app| app.into())
        .collect()
}

/// Clean the specified items (move to trash).
#[uniffi::export]
pub fn clean_items(paths: Vec<String>, options: FFICleanOptions) -> FFICleanResult {
    use std::time::Instant;

    let start = Instant::now();
    let config = runtime_config();
    let cleaners = get_cleaners_with_config(&config);
    let mut scan_index = build_scan_index(&cleaners, config.general.keep_recent_days);

    // Create trash mover
    let mover = match TrashMover::default_mover() {
        Ok(m) => m,
        Err(e) => {
            return FFICleanResult {
                category: "manual".to_string(),
                items_cleaned: 0,
                bytes_freed: 0,
                failures: vec![FFICleanFailure {
                    path: "".to_string(),
                    reason: format!("Failed to initialize trash mover: {}", e),
                }],
                clean_duration_ms: start.elapsed().as_millis() as u64,
            };
        }
    };

    let mut items_cleaned = 0u32;
    let mut bytes_freed = 0u64;
    let mut failures = Vec::new();
    let mut manual_paths = Vec::new();
    let mut grouped_items: HashMap<usize, Vec<CleanableItem>> = HashMap::new();

    for path_str in paths {
        if let Some((cleaner_idx, item)) = scan_index.remove(&path_str) {
            if item.requires_force && !options.force {
                failures.push(FFICleanFailure {
                    path: path_str,
                    reason: "Requires --force flag".to_string(),
                });
                continue;
            }
            grouped_items.entry(cleaner_idx).or_default().push(item);
        } else {
            manual_paths.push(path_str);
        }
    }

    let cleaner_ctx = CleanerContext {
        dry_run: options.dry_run,
        force: options.force,
        keep_recent_days: config.general.keep_recent_days,
        min_size: None,
        max_items: None,
        skip_confirm: true,
    };

    for (cleaner_idx, cleaner_items) in grouped_items {
        let result = cleaners[cleaner_idx].clean(&cleaner_items, &cleaner_ctx);
        items_cleaned += result.items_cleaned as u32;
        bytes_freed += result.bytes_freed;
        failures.extend(
            result
                .items_failed
                .into_iter()
                .map(|(path, reason)| FFICleanFailure {
                    path: path.to_string_lossy().to_string(),
                    reason,
                }),
        );
    }

    // Fallback: manually clean paths that don't belong to a cleaner category
    // (e.g. large files / duplicate files / downloads).
    for path_str in manual_paths {
        let path = PathBuf::from(&path_str);

        if crate::safety::is_protected(&path) {
            failures.push(FFICleanFailure {
                path: path_str,
                reason: format!("SAFETY VIOLATION: '{}' is protected", path.display()),
            });
            continue;
        }

        if options.dry_run {
            if path.exists() {
                bytes_freed += path_size(&path);
                items_cleaned += 1;
            } else {
                failures.push(FFICleanFailure {
                    path: path_str,
                    reason: "Path not found".to_string(),
                });
            }
            continue;
        }

        // Get size before cleaning.
        let size = path_size(&path);

        match mover.move_to_trash(&path) {
            Ok(_) => {
                items_cleaned += 1;
                bytes_freed += size;
            }
            Err(e) => {
                failures.push(FFICleanFailure {
                    path: path_str,
                    reason: e.to_string(),
                });
            }
        }
    }

    FFICleanResult {
        category: "mixed".to_string(),
        items_cleaned,
        bytes_freed,
        failures,
        clean_duration_ms: start.elapsed().as_millis() as u64,
    }
}

/// Uninstall an application and optionally its leftovers.
#[uniffi::export]
pub fn uninstall_app(app_path: String, include_leftovers: bool) -> FFICleanResult {
    use std::time::Instant;

    let start = Instant::now();
    let path = PathBuf::from(&app_path);

    // Create trash mover
    let mover = match TrashMover::default_mover() {
        Ok(m) => m,
        Err(e) => {
            return FFICleanResult {
                category: "uninstall".to_string(),
                items_cleaned: 0,
                bytes_freed: 0,
                failures: vec![FFICleanFailure {
                    path: app_path,
                    reason: format!("Failed to initialize trash mover: {}", e),
                }],
                clean_duration_ms: start.elapsed().as_millis() as u64,
            };
        }
    };

    let mut items_cleaned = 0u32;
    let mut bytes_freed = 0u64;
    let mut failures = Vec::new();

    // First, find the app to get its leftovers
    let apps = apps::scan_applications();
    let app = apps.iter().find(|a| a.path == path);

    // Get app size
    let app_size = if path.exists() {
        walkdir::WalkDir::new(&path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum()
    } else {
        0
    };

    // Move app to trash
    if path.exists() {
        match mover.move_to_trash(&path) {
            Ok(_) => {
                items_cleaned += 1;
                bytes_freed += app_size;
            }
            Err(e) => {
                failures.push(FFICleanFailure {
                    path: app_path.clone(),
                    reason: e.to_string(),
                });
            }
        }
    }

    // Clean leftovers if requested
    if include_leftovers {
        if let Some(app) = app {
            for leftover in &app.leftovers {
                let leftover_size = if leftover.is_dir() {
                    walkdir::WalkDir::new(leftover)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter_map(|e| e.metadata().ok())
                        .map(|m| m.len())
                        .sum()
                } else {
                    std::fs::metadata(leftover).map(|m| m.len()).unwrap_or(0)
                };

                match mover.move_to_trash(leftover) {
                    Ok(_) => {
                        items_cleaned += 1;
                        bytes_freed += leftover_size;
                    }
                    Err(e) => {
                        failures.push(FFICleanFailure {
                            path: leftover.to_string_lossy().to_string(),
                            reason: e.to_string(),
                        });
                    }
                }
            }
        }
    }

    FFICleanResult {
        category: "uninstall".to_string(),
        items_cleaned,
        bytes_freed,
        failures,
        clean_duration_ms: start.elapsed().as_millis() as u64,
    }
}

/// Get root disk information.
#[uniffi::export]
pub fn get_root_disk_info() -> Option<FFIDiskInfo> {
    disk::get_root_disk_info().map(|d| d.into())
}

/// Get all disk information.
#[uniffi::export]
pub fn get_all_disk_info() -> Vec<FFIDiskInfo> {
    use crate::platform::DiskInfoProvider;
    DiskInfoProvider::list_mounted_disks()
        .into_iter()
        .map(|d| d.into())
        .collect()
}

/// Check if a path is protected.
#[uniffi::export]
pub fn is_protected_path(path: String) -> bool {
    protected_paths::is_protected(&PathBuf::from(path))
}

/// Format a byte size for display.
#[uniffi::export]
pub fn format_size(bytes: u64) -> String {
    bytesize::ByteSize(bytes).to_string()
}

// ============================================================================
// New Scanner Functions
// ============================================================================

/// Scan for large files in user directories.
#[uniffi::export]
pub fn scan_large_files(options: FFILargeFileScanOptions) -> Vec<FFICleanableItem> {
    use crate::scanner::large_files;

    let opts = large_files::LargeFileScanOptions {
        min_size_bytes: options.min_size_bytes,
        max_results: options.max_results,
        include_hidden: options.include_hidden,
    };

    large_files::scan_large_files(&opts)
}

/// Scan for duplicate files.
#[uniffi::export]
pub fn scan_duplicates(options: FFIDuplicateScanOptions) -> FFIDuplicateScanResult {
    use crate::scanner::duplicates;

    let opts = duplicates::DuplicateScanOptions {
        min_size_bytes: options.min_size_bytes,
        max_groups: options.max_groups,
        include_hidden: false,
    };

    let result = duplicates::scan_duplicates(&opts);

    FFIDuplicateScanResult {
        groups: result
            .groups
            .into_iter()
            .map(|g| {
                let paths = g.path_strings();
                FFIDuplicateGroup {
                    hash: g.hash,
                    size: g.size,
                    wasted_size: g.wasted_size,
                    paths,
                    count: g.count,
                }
            })
            .collect(),
        total_wasted: result.total_wasted,
        files_scanned: result.files_scanned,
    }
}

/// Scan the Downloads folder.
#[uniffi::export]
pub fn scan_downloads(options: FFIDownloadsScanOptions) -> FFIDownloadsScanResult {
    use crate::scanner::downloads;

    let opts = downloads::DownloadsScanOptions {
        old_threshold_days: options.old_threshold_days,
        include_hidden: options.include_hidden,
    };

    let result = downloads::scan_downloads(&opts);

    FFIDownloadsScanResult {
        items: result.items.iter().map(|i| i.to_ffi()).collect(),
        total_size: result.total_size,
        size_by_category: result
            .size_by_category
            .iter()
            .map(|(cat, size)| FFICategorySize {
                category: match cat {
                    downloads::DownloadCategory::OldFiles => FFIDownloadCategory::OldFiles,
                    downloads::DownloadCategory::Archives => FFIDownloadCategory::Archives,
                    downloads::DownloadCategory::LargeMedia => FFIDownloadCategory::LargeMedia,
                    downloads::DownloadCategory::Documents => FFIDownloadCategory::Documents,
                    downloads::DownloadCategory::Installers => FFIDownloadCategory::Installers,
                    downloads::DownloadCategory::Other => FFIDownloadCategory::Other,
                },
                size: *size,
            })
            .collect(),
    }
}
