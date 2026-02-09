//! Cross-platform disk information provider.
//!
//! This module provides platform-specific implementations for disk utilities:
//! - macOS: Uses `diskutil` and `df` commands
//! - Linux: Uses `df` and `/proc/mounts`
//! - Windows: Uses Windows API (GetDiskFreeSpaceExW)

use crate::disk::{DiskInfo, SmartStatus};

/// Cross-platform disk information provider.
pub struct DiskInfoProvider;

impl DiskInfoProvider {
    /// Get disk info for a mount point.
    #[cfg(target_os = "macos")]
    pub fn get_disk_info(mount_point: &str) -> Option<DiskInfo> {
        crate::disk::diskutil::get_disk_info(mount_point)
    }

    /// Get disk info for a mount point.
    #[cfg(target_os = "linux")]
    pub fn get_disk_info(mount_point: &str) -> Option<DiskInfo> {
        use std::process::Command;

        let output = Command::new("df").args(["-k", mount_point]).output().ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        if lines.len() < 2 {
            return None;
        }

        let parts: Vec<&str> = lines[1].split_whitespace().collect();
        if parts.len() < 6 {
            return None;
        }

        let device_node = parts[0].to_string();
        let total_kb: u64 = parts[1].parse().unwrap_or(0);
        let available_kb: u64 = parts[3].parse().unwrap_or(0);
        let used_kb = total_kb.saturating_sub(available_kb);

        // Get filesystem type from /proc/mounts
        let filesystem =
            Self::get_linux_filesystem(&device_node).unwrap_or_else(|| "Unknown".to_string());

        // Get volume name - use mount point base name
        let name = std::path::Path::new(mount_point)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| {
                if mount_point == "/" {
                    "Root".to_string()
                } else {
                    mount_point.to_string()
                }
            });

        Some(DiskInfo {
            name,
            mount_point: mount_point.to_string(),
            device_node,
            filesystem,
            total_bytes: total_kb * 1024,
            used_bytes: used_kb * 1024,
            free_bytes: available_kb * 1024,
        })
    }

    /// Get disk info for a mount point (drive letter on Windows).
    #[cfg(target_os = "windows")]
    pub fn get_disk_info(mount_point: &str) -> Option<DiskInfo> {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        // Ensure the path ends with backslash for Windows API
        let path = if mount_point.ends_with('\\') {
            mount_point.to_string()
        } else {
            format!("{}\\", mount_point)
        };

        let wide_path: Vec<u16> = OsStr::new(&path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut free_bytes_available: u64 = 0;
        let mut total_bytes: u64 = 0;
        let mut total_free_bytes: u64 = 0;

        let success = unsafe {
            windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
                wide_path.as_ptr(),
                &mut free_bytes_available as *mut u64,
                &mut total_bytes as *mut u64,
                &mut total_free_bytes as *mut u64,
            )
        };

        if success == 0 {
            return None;
        }

        // Get drive name
        let name = if mount_point.len() >= 2 && mount_point.chars().nth(1) == Some(':') {
            format!("Drive {}", mount_point.chars().next().unwrap_or('?'))
        } else {
            mount_point.to_string()
        };

        Some(DiskInfo {
            name,
            mount_point: mount_point.to_string(),
            device_node: mount_point.to_string(),
            filesystem: Self::get_windows_filesystem(mount_point)
                .unwrap_or_else(|| "NTFS".to_string()),
            total_bytes,
            used_bytes: total_bytes.saturating_sub(free_bytes_available),
            free_bytes: free_bytes_available,
        })
    }

    /// Get filesystem type for a device on Linux.
    #[cfg(target_os = "linux")]
    fn get_linux_filesystem(device: &str) -> Option<String> {
        let content = std::fs::read_to_string("/proc/mounts").ok()?;
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && parts[0] == device {
                return Some(parts[2].to_string());
            }
        }
        None
    }

    /// Get filesystem type for a drive on Windows.
    #[cfg(target_os = "windows")]
    fn get_windows_filesystem(_drive: &str) -> Option<String> {
        // For now, return a default. Could use GetVolumeInformationW for actual value.
        Some("NTFS".to_string())
    }

    /// List all mounted disks.
    #[cfg(target_os = "macos")]
    pub fn list_mounted_disks() -> Vec<DiskInfo> {
        crate::disk::diskutil::list_mounted_disks()
    }

    /// List all mounted disks.
    #[cfg(target_os = "linux")]
    pub fn list_mounted_disks() -> Vec<DiskInfo> {
        let mut disks = Vec::new();

        if let Ok(content) = std::fs::read_to_string("/proc/mounts") {
            for line in content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let device = parts[0];
                    let mount_point = parts[1];

                    // Only include real disks (skip virtual filesystems)
                    if device.starts_with("/dev/sd")
                        || device.starts_with("/dev/nvme")
                        || device.starts_with("/dev/vd")
                        || device.starts_with("/dev/xvd")
                        || device.starts_with("/dev/hd")
                    {
                        if let Some(disk) = Self::get_disk_info(mount_point) {
                            disks.push(disk);
                        }
                    }
                }
            }
        }

        // If no disks found, at least try to get root
        if disks.is_empty() {
            if let Some(root) = Self::get_disk_info("/") {
                disks.push(root);
            }
        }

        disks
    }

    /// List all mounted disks.
    #[cfg(target_os = "windows")]
    pub fn list_mounted_disks() -> Vec<DiskInfo> {
        let mut disks = Vec::new();

        // Check common drive letters
        for letter in b'A'..=b'Z' {
            let drive = format!("{}:\\", letter as char);
            let path = std::path::Path::new(&drive);
            if path.exists() {
                if let Some(disk) = Self::get_disk_info(&drive) {
                    disks.push(disk);
                }
            }
        }

        disks
    }

    /// Get SMART status for a device.
    #[cfg(target_os = "macos")]
    pub fn get_smart_status(device: &str) -> SmartStatus {
        crate::disk::diskutil::get_smart_status(device)
    }

    /// Get SMART status for a device.
    /// On Linux, we rely on smartctl if available.
    #[cfg(target_os = "linux")]
    pub fn get_smart_status(device: &str) -> SmartStatus {
        if crate::disk::smartctl::is_available() {
            match crate::disk::smartctl::get_smart_data(device) {
                Ok(_) => SmartStatus::Verified,
                Err(_) => SmartStatus::Unknown,
            }
        } else {
            SmartStatus::NotSupported
        }
    }

    /// Get SMART status for a device.
    /// On Windows, SMART access typically requires admin privileges.
    #[cfg(target_os = "windows")]
    pub fn get_smart_status(_device: &str) -> SmartStatus {
        // SMART access on Windows requires admin privileges and WMI or direct disk access
        SmartStatus::NotSupported
    }

    /// Get the root/main disk.
    pub fn get_root_disk() -> Option<DiskInfo> {
        #[cfg(target_os = "windows")]
        {
            Self::get_disk_info("C:\\")
        }
        #[cfg(not(target_os = "windows"))]
        {
            Self::get_disk_info("/")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_root_disk() {
        // This should work on any platform
        let disk = DiskInfoProvider::get_root_disk();
        // May fail in some CI environments, so we just check it doesn't panic
        if let Some(d) = disk {
            assert!(d.total_bytes > 0);
        }
    }

    #[test]
    fn test_list_mounted_disks() {
        let disks = DiskInfoProvider::list_mounted_disks();
        // Should find at least one disk on most systems
        // May be empty in some CI environments
        for disk in &disks {
            assert!(!disk.mount_point.is_empty());
        }
    }
}
