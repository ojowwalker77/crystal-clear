//! Native macOS diskutil interface.

use std::process::Command;

use super::{DiskInfo, SmartStatus};

/// Get disk info for a mount point.
pub fn get_disk_info(mount_point: &str) -> Option<DiskInfo> {
    // Use df to get disk space info
    let output = Command::new("df")
        .args(["-k", mount_point])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    if lines.len() < 2 {
        return None;
    }

    // Parse df output (skip header)
    let parts: Vec<&str> = lines[1].split_whitespace().collect();
    if parts.len() < 6 {
        return None;
    }

    let device_node = parts[0].to_string();
    let total_kb: u64 = parts[1].parse().unwrap_or(0);
    // Note: df's "Used" column (parts[2]) doesn't include APFS snapshots/purgeable space
    // So we calculate used = total - available for accurate reporting
    let available_kb: u64 = parts[3].parse().unwrap_or(0);
    let used_kb: u64 = total_kb.saturating_sub(available_kb);
    let free_kb: u64 = available_kb;

    // Get more info from diskutil
    let (name, filesystem) = get_diskutil_info(&device_node).unwrap_or_else(|| {
        ("Unknown".to_string(), "Unknown".to_string())
    });

    Some(DiskInfo {
        name,
        mount_point: mount_point.to_string(),
        device_node,
        filesystem,
        total_bytes: total_kb * 1024,
        used_bytes: used_kb * 1024,
        free_bytes: free_kb * 1024,
    })
}

/// Get additional info from diskutil.
fn get_diskutil_info(device: &str) -> Option<(String, String)> {
    let output = Command::new("diskutil")
        .args(["info", device])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut name = String::new();
    let mut filesystem = String::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.starts_with("Volume Name:") {
            name = line
                .strip_prefix("Volume Name:")
                .unwrap_or("")
                .trim()
                .to_string();
        } else if line.starts_with("Type (Bundle):") {
            filesystem = line
                .strip_prefix("Type (Bundle):")
                .unwrap_or("")
                .trim()
                .to_string();
        } else if line.starts_with("File System Personality:") && filesystem.is_empty() {
            filesystem = line
                .strip_prefix("File System Personality:")
                .unwrap_or("")
                .trim()
                .to_string();
        }
    }

    if name.is_empty() {
        name = "Unknown".to_string();
    }
    if filesystem.is_empty() {
        filesystem = "Unknown".to_string();
    }

    Some((name, filesystem))
}

/// List all mounted disks.
pub fn list_mounted_disks() -> Vec<DiskInfo> {
    let mut disks = Vec::new();

    // Use df to list all mounted filesystems
    let output = match Command::new("df").args(["-k"]).output() {
        Ok(o) => o,
        Err(_) => return disks,
    };

    if !output.status.success() {
        return disks;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines().skip(1) {
        // Skip header
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 6 {
            continue;
        }

        let device = parts[0];

        // Skip non-disk devices
        if !device.starts_with("/dev/disk") {
            continue;
        }

        // Get the mount point (last field, may contain spaces)
        let mount_point = if parts.len() >= 9 {
            parts[8..].join(" ")
        } else {
            parts.last().unwrap_or(&"").to_string()
        };

        if let Some(disk) = get_disk_info(&mount_point) {
            disks.push(disk);
        }
    }

    // If no disks found, at least try to get root
    if disks.is_empty() {
        if let Some(root) = get_disk_info("/") {
            disks.push(root);
        }
    }

    disks
}

/// Get SMART status using diskutil.
pub fn get_smart_status(device: &str) -> SmartStatus {
    let output = match Command::new("diskutil").args(["info", device]).output() {
        Ok(o) => o,
        Err(_) => return SmartStatus::Unknown,
    };

    if !output.status.success() {
        return SmartStatus::Unknown;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let line = line.trim();
        if line.starts_with("SMART Status:") {
            let status = line
                .strip_prefix("SMART Status:")
                .unwrap_or("")
                .trim()
                .to_lowercase();

            return match status.as_str() {
                "verified" => SmartStatus::Verified,
                "failing" => SmartStatus::Failing,
                "not supported" => SmartStatus::NotSupported,
                _ => SmartStatus::Unknown,
            };
        }
    }

    SmartStatus::NotSupported
}
