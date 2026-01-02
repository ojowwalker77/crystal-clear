//! Disk health monitoring module.
//!
//! Provides disk information and SMART monitoring using:
//! - Native macOS `diskutil` (macOS only)
//! - `df` command (Linux)
//! - Windows API (Windows)
//! - `smartctl` from smartmontools (if installed, all platforms)

#[cfg(target_os = "macos")]
pub mod diskutil;
pub mod smartctl;

use serde::{Deserialize, Serialize};

/// Basic disk information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    /// Disk name (e.g., "Macintosh HD").
    pub name: String,
    /// Mount point (e.g., "/").
    pub mount_point: String,
    /// Device node (e.g., "/dev/disk0s2").
    pub device_node: String,
    /// Filesystem type (e.g., "APFS", "HFS+").
    pub filesystem: String,
    /// Total size in bytes.
    pub total_bytes: u64,
    /// Used space in bytes.
    pub used_bytes: u64,
    /// Free space in bytes.
    pub free_bytes: u64,
}

/// SMART health status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SmartStatus {
    /// SMART status verified/healthy.
    Verified,
    /// SMART status failing.
    Failing,
    /// SMART status unknown.
    Unknown,
    /// SMART not supported on this disk.
    NotSupported,
}

/// Detailed SMART data.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SmartData {
    /// Temperature in Celsius.
    pub temperature_celsius: Option<u32>,
    /// Power on hours.
    pub power_on_hours: Option<u64>,
    /// Power cycle count.
    pub power_cycles: Option<u64>,
    /// Percentage of life used (SSD wear indicator).
    pub percentage_used: Option<u8>,
    /// Available spare percentage.
    pub available_spare: Option<u8>,
    /// Read error rate.
    pub read_error_rate: Option<u64>,
    /// Write error rate.
    pub write_error_rate: Option<u64>,
    /// Reallocated sector count.
    pub reallocated_sectors: Option<u64>,
    /// Pending sector count.
    pub pending_sectors: Option<u64>,
    /// Uncorrectable sector count.
    pub uncorrectable_sectors: Option<u64>,
    /// Data units read (in 512-byte units).
    pub data_units_read: Option<u64>,
    /// Data units written (in 512-byte units).
    pub data_units_written: Option<u64>,
    /// Unsafe shutdowns count.
    pub unsafe_shutdowns: Option<u64>,
    /// Media errors count.
    pub media_errors: Option<u64>,
}

/// Combined disk health information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskHealth {
    /// Basic disk information.
    pub disk: DiskInfo,
    /// SMART status.
    pub smart_status: SmartStatus,
    /// Detailed SMART data (if available).
    pub smart_data: Option<SmartData>,
}

/// Get information about the root disk.
pub fn get_root_disk_info() -> Option<DiskInfo> {
    crate::platform::DiskInfoProvider::get_root_disk()
}

/// Get health information for all disks.
pub fn get_all_disk_health() -> Vec<DiskHealth> {
    use crate::platform::DiskInfoProvider;

    let mut results = Vec::new();

    // Get list of mounted volumes using platform abstraction
    let disks = DiskInfoProvider::list_mounted_disks();

    for disk in disks {
        // Get SMART status using platform abstraction
        let smart_status = DiskInfoProvider::get_smart_status(&disk.device_node);

        // Try to get detailed SMART data if smartctl is available
        let smart_data = if smartctl::is_available() {
            smartctl::get_smart_data(&disk.device_node).ok()
        } else {
            None
        };

        results.push(DiskHealth {
            disk,
            smart_status,
            smart_data,
        });
    }

    results
}
