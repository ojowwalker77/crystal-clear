//! smartctl (smartmontools) interface for detailed SMART data.

use std::process::Command;

use super::SmartData;

/// Check if smartctl is available.
pub fn is_available() -> bool {
    Command::new("which")
        .arg("smartctl")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get detailed SMART data using smartctl.
pub fn get_smart_data(device: &str) -> Result<SmartData, String> {
    // Extract base device (e.g., /dev/disk0 from /dev/disk0s2)
    let base_device = get_base_device(device);

    // Try JSON output first (newer smartctl versions)
    let output = Command::new("smartctl")
        .args(["-a", &base_device, "--json"])
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        if let Ok(data) = parse_json_output(&output.stdout) {
            return Ok(data);
        }
    }

    // Fall back to text parsing
    let output = Command::new("smartctl")
        .args(["-a", &base_device])
        .output()
        .map_err(|e| e.to_string())?;

    // Note: smartctl may return non-zero for various reasons
    // but still provide useful output
    parse_text_output(&output.stdout)
}

/// Get base device from a partition device.
fn get_base_device(device: &str) -> String {
    // Convert /dev/disk0s2 to /dev/disk0
    if device.starts_with("/dev/disk") {
        if let Some(pos) = device.find('s') {
            if pos > 9 {
                // After "disk" + number
                return device[..pos].to_string();
            }
        }
    }
    device.to_string()
}

/// Parse JSON output from smartctl.
fn parse_json_output(stdout: &[u8]) -> Result<SmartData, String> {
    let json: serde_json::Value = serde_json::from_slice(stdout).map_err(|e| e.to_string())?;

    let mut data = SmartData::default();

    // Temperature
    if let Some(temp) = json.get("temperature").and_then(|t| t.get("current")) {
        data.temperature_celsius = temp.as_u64().map(|v| v as u32);
    }

    // Power on time
    if let Some(pot) = json.get("power_on_time").and_then(|t| t.get("hours")) {
        data.power_on_hours = pot.as_u64();
    }

    // Power cycles
    if let Some(pc) = json.get("power_cycle_count") {
        data.power_cycles = pc.as_u64();
    }

    // NVMe specific attributes
    if let Some(nvme) = json.get("nvme_smart_health_information_log") {
        data.percentage_used = nvme
            .get("percentage_used")
            .and_then(|v| v.as_u64())
            .map(|v| v as u8);

        data.available_spare = nvme
            .get("available_spare")
            .and_then(|v| v.as_u64())
            .map(|v| v as u8);

        data.data_units_read = nvme.get("data_units_read").and_then(|v| v.as_u64());

        data.data_units_written = nvme.get("data_units_written").and_then(|v| v.as_u64());

        data.unsafe_shutdowns = nvme.get("unsafe_shutdowns").and_then(|v| v.as_u64());

        data.media_errors = nvme.get("media_errors").and_then(|v| v.as_u64());
    }

    // ATA attributes
    if let Some(attrs) = json
        .get("ata_smart_attributes")
        .and_then(|a| a.get("table"))
    {
        if let Some(arr) = attrs.as_array() {
            for attr in arr {
                let id = attr.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
                let raw_value = attr
                    .get("raw")
                    .and_then(|r| r.get("value"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                match id {
                    1 => data.read_error_rate = Some(raw_value),
                    5 => data.reallocated_sectors = Some(raw_value),
                    9 => data.power_on_hours = Some(raw_value),
                    12 => data.power_cycles = Some(raw_value),
                    194 | 190 => data.temperature_celsius = Some(raw_value as u32),
                    197 => data.pending_sectors = Some(raw_value),
                    198 => data.uncorrectable_sectors = Some(raw_value),
                    _ => {}
                }
            }
        }
    }

    Ok(data)
}

/// Parse text output from smartctl.
fn parse_text_output(stdout: &[u8]) -> Result<SmartData, String> {
    let text = String::from_utf8_lossy(stdout);
    let mut data = SmartData::default();

    for line in text.lines() {
        let line = line.trim();

        // Temperature
        if line.contains("Temperature:") || line.contains("Temperature_Celsius") {
            if let Some(temp) = extract_number(line) {
                data.temperature_celsius = Some(temp as u32);
            }
        }

        // Power on hours
        if line.contains("Power_On_Hours") || line.contains("Power On Hours:") {
            if let Some(hours) = extract_number(line) {
                data.power_on_hours = Some(hours);
            }
        }

        // Power cycles
        if line.contains("Power_Cycle_Count") || line.contains("Power Cycles:") {
            if let Some(cycles) = extract_number(line) {
                data.power_cycles = Some(cycles);
            }
        }

        // Percentage used (NVMe)
        if line.contains("Percentage Used:") {
            if let Some(pct) = extract_number(line) {
                data.percentage_used = Some(pct as u8);
            }
        }

        // Available spare (NVMe)
        if line.contains("Available Spare:") {
            if let Some(spare) = extract_number(line) {
                data.available_spare = Some(spare as u8);
            }
        }

        // Reallocated sectors
        if line.contains("Reallocated_Sector") {
            if let Some(sectors) = extract_last_number(line) {
                data.reallocated_sectors = Some(sectors);
            }
        }

        // Unsafe shutdowns
        if line.contains("Unsafe Shutdowns:") {
            if let Some(count) = extract_number(line) {
                data.unsafe_shutdowns = Some(count);
            }
        }

        // Media errors
        if line.contains("Media and Data Integrity Errors:") {
            if let Some(errors) = extract_number(line) {
                data.media_errors = Some(errors);
            }
        }
    }

    Ok(data)
}

/// Extract the first number from a string.
fn extract_number(s: &str) -> Option<u64> {
    s.split_whitespace()
        .filter_map(|word| {
            word.trim_end_matches('%')
                .trim_end_matches('C')
                .parse::<u64>()
                .ok()
        })
        .next()
}

/// Extract the last number from a string (useful for ATA attributes).
fn extract_last_number(s: &str) -> Option<u64> {
    s.split_whitespace()
        .filter_map(|word| word.parse::<u64>().ok())
        .last()
}
