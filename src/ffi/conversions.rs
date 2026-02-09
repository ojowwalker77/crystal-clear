//! Type conversions between internal Rust types and FFI-safe types.

use crate::cleaners::{
    CleanResult, CleanableItem, CleanerCategory, ItemType, RiskLevel, ScanResult,
};
use crate::disk::DiskInfo;
use crate::error::CleanmacError;
use crate::scanner::apps::InstalledApp;

use super::types::*;

// ItemType conversions
impl From<ItemType> for FFIItemType {
    fn from(item_type: ItemType) -> Self {
        match item_type {
            ItemType::File => FFIItemType::File,
            ItemType::Directory => FFIItemType::Directory,
            ItemType::Symlink => FFIItemType::Symlink,
        }
    }
}

// RiskLevel conversions
impl From<RiskLevel> for FFIRiskLevel {
    fn from(risk_level: RiskLevel) -> Self {
        match risk_level {
            RiskLevel::Low => FFIRiskLevel::Low,
            RiskLevel::Medium => FFIRiskLevel::Medium,
            RiskLevel::High => FFIRiskLevel::High,
        }
    }
}

// CleanerCategory conversions
impl From<CleanerCategory> for FFICleanerCategory {
    fn from(category: CleanerCategory) -> Self {
        match category {
            CleanerCategory::System => FFICleanerCategory::System,
            CleanerCategory::Developer => FFICleanerCategory::Developer,
            CleanerCategory::Apps => FFICleanerCategory::Apps,
        }
    }
}

// CleanableItem conversions
impl From<CleanableItem> for FFICleanableItem {
    fn from(item: CleanableItem) -> Self {
        Self {
            path: item.path.to_string_lossy().to_string(),
            size: item.size,
            item_type: item.item_type.into(),
            age_days: item.age_days,
            description: item.description,
            requires_force: item.requires_force,
            risk_level: item.risk_level.into(),
        }
    }
}

// ScanResult conversions
impl From<ScanResult> for FFIScanResult {
    fn from(result: ScanResult) -> Self {
        Self {
            category: result.category.clone(),
            category_name: result.category, // Use same for now, can add display name mapping
            items: result
                .items
                .into_iter()
                .map(FFICleanableItem::from)
                .collect(),
            total_size: result.total_size,
            scan_duration_ms: result.scan_duration.as_millis() as u64,
            errors: result.errors,
        }
    }
}

// CleanResult conversions
impl From<CleanResult> for FFICleanResult {
    fn from(result: CleanResult) -> Self {
        Self {
            category: result.category,
            items_cleaned: result.items_cleaned as u32,
            bytes_freed: result.bytes_freed,
            failures: result
                .items_failed
                .into_iter()
                .map(|(path, reason)| FFICleanFailure {
                    path: path.to_string_lossy().to_string(),
                    reason,
                })
                .collect(),
            clean_duration_ms: result.clean_duration.as_millis() as u64,
        }
    }
}

// InstalledApp conversions
impl From<InstalledApp> for FFIInstalledApp {
    fn from(app: InstalledApp) -> Self {
        Self {
            name: app.name,
            path: app.path.to_string_lossy().to_string(),
            size: app.size,
            bundle_id: app.bundle_id,
            leftover_paths: app
                .leftovers
                .into_iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
            leftover_size: app.leftover_size,
        }
    }
}

// DiskInfo conversions
impl From<DiskInfo> for FFIDiskInfo {
    fn from(disk: DiskInfo) -> Self {
        Self {
            name: disk.name,
            mount_point: disk.mount_point,
            device_node: disk.device_node,
            filesystem: disk.filesystem,
            total_bytes: disk.total_bytes,
            used_bytes: disk.used_bytes,
            free_bytes: disk.free_bytes,
        }
    }
}

// Error conversions
impl From<CleanmacError> for FFIError {
    fn from(error: CleanmacError) -> Self {
        match error {
            CleanmacError::ProtectedPath { path } => FFIError::ProtectedPath {
                path: path.to_string_lossy().to_string(),
            },
            CleanmacError::BoundaryEscape { path, resolved } => FFIError::BoundaryEscape {
                path: path.to_string_lossy().to_string(),
                resolved: resolved.to_string_lossy().to_string(),
            },
            CleanmacError::PermissionDenied { path } => FFIError::PermissionDenied {
                path: path.to_string_lossy().to_string(),
            },
            CleanmacError::PathNotFound { path } => FFIError::PathNotFound {
                path: path.to_string_lossy().to_string(),
            },
            CleanmacError::TrashFailed { path, reason } => FFIError::TrashFailed {
                path: path.to_string_lossy().to_string(),
                reason,
            },
            CleanmacError::Config { message } => FFIError::ConfigError { message },
            _ => FFIError::CleanFailed {
                message: error.to_string(),
            },
        }
    }
}
