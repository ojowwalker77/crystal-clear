//! Size calculation utilities.

use std::path::Path;
use walkdir::WalkDir;

/// Calculate the size of a file or directory.
pub fn calculate_size(path: &Path) -> u64 {
    // Use symlink_metadata to avoid following symlinks (consistent with calculate_dir_size)
    if let Ok(meta) = path.symlink_metadata() {
        if meta.is_file() {
            meta.len()
        } else if meta.is_dir() {
            calculate_dir_size(path)
        } else {
            // Symlinks: just return 0 (the symlink itself is tiny)
            0
        }
    } else {
        0
    }
}

/// Calculate the total size of a directory recursively.
pub fn calculate_dir_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .follow_links(false) // Don't follow symlinks to avoid counting outside files
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

/// Format bytes as human-readable string.
pub fn format_size(bytes: u64) -> String {
    bytesize::ByteSize(bytes).to_string()
}

/// Parse a human-readable size string (e.g., "10GB", "500MB").
pub fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim().to_uppercase();

    let (num_str, multiplier) = if s.ends_with("TB") {
        (&s[..s.len() - 2], 1024u64 * 1024 * 1024 * 1024)
    } else if s.ends_with("GB") {
        (&s[..s.len() - 2], 1024u64 * 1024 * 1024)
    } else if s.ends_with("MB") {
        (&s[..s.len() - 2], 1024u64 * 1024)
    } else if s.ends_with("KB") {
        (&s[..s.len() - 2], 1024u64)
    } else if s.ends_with("B") {
        (&s[..s.len() - 1], 1u64)
    } else {
        // Assume bytes if no suffix
        (s.as_str(), 1u64)
    };

    num_str.trim().parse::<f64>().ok().and_then(|n| {
        if n < 0.0 {
            None // Reject negative values
        } else {
            Some((n * multiplier as f64) as u64)
        }
    })
}

/// Get the age of a file in days.
pub fn get_age_days(path: &Path) -> Option<u32> {
    path.metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.elapsed().ok())
        .map(|d| (d.as_secs() / 86400) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_file_size() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("test.txt");
        fs::write(&file, "hello world").unwrap();

        let size = calculate_size(&file);
        assert_eq!(size, 11); // "hello world" = 11 bytes
    }

    #[test]
    fn test_dir_size() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join("testdir");
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("file1.txt"), "hello").unwrap();
        fs::write(dir.join("file2.txt"), "world").unwrap();

        let size = calculate_size(&dir);
        assert_eq!(size, 10); // "hello" + "world" = 10 bytes
    }

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("100"), Some(100));
        assert_eq!(parse_size("100B"), Some(100));
        assert_eq!(parse_size("1KB"), Some(1024));
        assert_eq!(parse_size("1MB"), Some(1024 * 1024));
        assert_eq!(parse_size("1GB"), Some(1024 * 1024 * 1024));
        assert_eq!(
            parse_size("1.5GB"),
            Some((1.5 * 1024.0 * 1024.0 * 1024.0) as u64)
        );
    }

    #[test]
    fn test_format_size() {
        // bytesize uses decimal (SI) units and shows KB up to ~10MB
        // 1024 bytes = "1.0 KB"
        // 10_000_000 bytes = "10.0 MB"
        assert!(format_size(1024).contains("KB") || format_size(1024).contains("K"));
        assert!(format_size(10_000_000).contains("MB") || format_size(10_000_000).contains("M"));
    }
}
