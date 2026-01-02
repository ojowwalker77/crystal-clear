//! Audit log writer with SQLite backend.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use chrono::{Duration, Utc};

use crate::audit::entry::AuditEntry;
use crate::db::Database;
use crate::error::CleanmacError;

/// Default audit log path (JSONL fallback).
pub fn default_audit_path() -> PathBuf {
    dirs::data_local_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("crystal-clear")
        .join("audit.log")
}

/// Audit logger that writes entries to SQLite (primary) with JSONL fallback.
pub struct AuditLogger {
    /// Database connection for primary storage.
    db: Option<Database>,
    /// Fallback JSONL path (for migration or if DB unavailable).
    jsonl_path: PathBuf,
    /// Whether logging is enabled.
    enabled: bool,
}

impl std::fmt::Debug for AuditLogger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuditLogger")
            .field("db", &self.db.is_some())
            .field("jsonl_path", &self.jsonl_path)
            .field("enabled", &self.enabled)
            .finish()
    }
}

impl AuditLogger {
    /// Create a new audit logger with optional SQLite backend.
    pub fn new(db: Option<Database>, jsonl_path: PathBuf, enabled: bool) -> Self {
        Self {
            db,
            jsonl_path,
            enabled,
        }
    }

    /// Create a logger with default paths and SQLite backend.
    pub fn default_logger(enabled: bool) -> Self {
        let jsonl_path = default_audit_path();
        let db = Database::open().ok();

        // If DB is new, migrate existing JSONL
        if let Some(ref database) = db {
            if let Err(e) = migrate_jsonl_to_sqlite(&jsonl_path, database) {
                tracing::warn!("JSONL migration failed: {}", e);
            }
        }

        Self::new(db, jsonl_path, enabled)
    }

    /// Create a logger with only JSONL backend (no SQLite).
    pub fn jsonl_only(path: PathBuf, enabled: bool) -> Self {
        Self::new(None, path, enabled)
    }

    /// Log an entry to the audit log.
    pub fn log(&self, entry: &AuditEntry) -> Result<(), CleanmacError> {
        if !self.enabled {
            return Ok(());
        }

        // Try SQLite first
        if let Some(ref db) = self.db {
            return db.insert_audit_entry(entry);
        }

        // Fallback to JSONL
        self.log_to_jsonl(entry)
    }

    fn log_to_jsonl(&self, entry: &AuditEntry) -> Result<(), CleanmacError> {
        // Ensure parent directory exists
        if let Some(parent) = self.jsonl_path.parent() {
            fs::create_dir_all(parent).map_err(|e| CleanmacError::Filesystem {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        // Serialize entry to JSON
        let json = serde_json::to_string(entry).map_err(|e| CleanmacError::Config {
            message: format!("Failed to serialize audit entry: {}", e),
        })?;

        // Append to log file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.jsonl_path)
            .map_err(|e| CleanmacError::Filesystem {
                path: self.jsonl_path.clone(),
                source: e,
            })?;

        writeln!(file, "{}", json).map_err(|e| CleanmacError::Filesystem {
            path: self.jsonl_path.clone(),
            source: e,
        })?;

        Ok(())
    }

    /// Log a cleaned item.
    pub fn log_cleaned(
        &self,
        cleaner_id: &str,
        path: &Path,
        size: u64,
    ) -> Result<(), CleanmacError> {
        self.log(&AuditEntry::cleaned(cleaner_id, path.to_path_buf(), size))
    }

    /// Log a dry-run item.
    pub fn log_dry_run(
        &self,
        cleaner_id: &str,
        path: &Path,
        size: u64,
    ) -> Result<(), CleanmacError> {
        self.log(&AuditEntry::dry_run(cleaner_id, path.to_path_buf(), size))
    }

    /// Log an error.
    pub fn log_error(
        &self,
        cleaner_id: &str,
        path: &Path,
        error: &str,
    ) -> Result<(), CleanmacError> {
        self.log(&AuditEntry::error(cleaner_id, path.to_path_buf(), error))
    }

    /// Read recent entries from the audit log.
    pub fn read_recent(&self, count: usize) -> Result<Vec<AuditEntry>, CleanmacError> {
        // Try SQLite first
        if let Some(ref db) = self.db {
            return db.get_recent_audit_entries(count);
        }

        // Fallback to JSONL
        self.read_recent_jsonl(count)
    }

    fn read_recent_jsonl(&self, count: usize) -> Result<Vec<AuditEntry>, CleanmacError> {
        if !self.jsonl_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.jsonl_path).map_err(|e| CleanmacError::Filesystem {
            path: self.jsonl_path.clone(),
            source: e,
        })?;

        let reader = BufReader::new(file);
        let mut entries: Vec<AuditEntry> = Vec::new();

        for line in reader.lines() {
            let line = line.map_err(|e| CleanmacError::Filesystem {
                path: self.jsonl_path.clone(),
                source: e,
            })?;

            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str(&line) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    tracing::warn!("Failed to parse audit entry: {}", e);
                }
            }
        }

        // Return the last `count` entries
        let start = entries.len().saturating_sub(count);
        Ok(entries[start..].to_vec())
    }

    /// Clear entries older than the specified number of days.
    pub fn cleanup_old_entries(&self, days: u32) -> Result<usize, CleanmacError> {
        // Try SQLite first
        if let Some(ref db) = self.db {
            return db.cleanup_old_audit_entries(days);
        }

        // Fallback to JSONL
        self.cleanup_old_entries_jsonl(days)
    }

    fn cleanup_old_entries_jsonl(&self, days: u32) -> Result<usize, CleanmacError> {
        if !self.jsonl_path.exists() {
            return Ok(0);
        }

        let cutoff = Utc::now() - Duration::days(days as i64);
        let file = File::open(&self.jsonl_path).map_err(|e| CleanmacError::Filesystem {
            path: self.jsonl_path.clone(),
            source: e,
        })?;

        let reader = BufReader::new(file);
        let mut retained: Vec<String> = Vec::new();
        let mut removed = 0;

        for line in reader.lines() {
            let line = line.map_err(|e| CleanmacError::Filesystem {
                path: self.jsonl_path.clone(),
                source: e,
            })?;

            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<AuditEntry>(&line) {
                Ok(entry) => {
                    if entry.timestamp >= cutoff {
                        retained.push(line);
                    } else {
                        removed += 1;
                    }
                }
                Err(_) => {
                    // Keep unparseable lines to avoid data loss
                    retained.push(line);
                }
            }
        }

        // Write retained entries back
        let mut file =
            File::create(&self.jsonl_path).map_err(|e| CleanmacError::Filesystem {
                path: self.jsonl_path.clone(),
                source: e,
            })?;

        for line in retained {
            writeln!(file, "{}", line).map_err(|e| CleanmacError::Filesystem {
                path: self.jsonl_path.clone(),
                source: e,
            })?;
        }

        Ok(removed)
    }

    /// Get the path to the audit log (JSONL fallback path).
    pub fn path(&self) -> &Path {
        &self.jsonl_path
    }

    /// Check if logging is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Check if SQLite backend is available.
    pub fn has_database(&self) -> bool {
        self.db.is_some()
    }
}

/// Migrate existing JSONL entries to SQLite.
fn migrate_jsonl_to_sqlite(jsonl_path: &Path, db: &Database) -> Result<usize, CleanmacError> {
    if !jsonl_path.exists() {
        return Ok(0);
    }

    // Check if we've already migrated (audit_log table has entries)
    let existing_count = db
        .get_recent_audit_entries(1)
        .map(|e| e.len())
        .unwrap_or(0);
    if existing_count > 0 {
        // Already has entries, skip migration
        return Ok(0);
    }

    let file = File::open(jsonl_path).map_err(|e| CleanmacError::Filesystem {
        path: jsonl_path.to_path_buf(),
        source: e,
    })?;

    let reader = BufReader::new(file);
    let mut count = 0;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        if line.trim().is_empty() {
            continue;
        }

        if let Ok(entry) = serde_json::from_str::<AuditEntry>(&line) {
            if db.insert_audit_entry(&entry).is_ok() {
                count += 1;
            }
        }
    }

    // Rename old file as backup if we migrated entries
    if count > 0 {
        let backup_path = jsonl_path.with_extension("log.migrated");
        if let Err(e) = fs::rename(jsonl_path, &backup_path) {
            tracing::warn!("Could not rename migrated JSONL file: {}", e);
        } else {
            tracing::info!("Migrated {} audit entries to SQLite", count);
        }
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_log_and_read_jsonl() {
        let temp = TempDir::new().unwrap();
        let log_path = temp.path().join("audit.log");

        // Test JSONL-only mode
        let logger = AuditLogger::jsonl_only(log_path.clone(), true);

        // Log some entries
        logger
            .log_cleaned("test", Path::new("/test/path"), 1024)
            .unwrap();
        logger
            .log_dry_run("test", Path::new("/test/path2"), 2048)
            .unwrap();
        logger
            .log_error("test", Path::new("/test/path3"), "test error")
            .unwrap();

        // Read back
        let entries = logger.read_recent(10).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(entries[0].success);
        assert!(entries[1].dry_run);
        assert!(!entries[2].success);
    }

    #[test]
    fn test_disabled_logger() {
        let temp = TempDir::new().unwrap();
        let log_path = temp.path().join("audit.log");

        let logger = AuditLogger::jsonl_only(log_path.clone(), false);

        // Log should succeed but not write
        logger
            .log_cleaned("test", Path::new("/test/path"), 1024)
            .unwrap();

        // File should not exist
        assert!(!log_path.exists());
    }

    #[test]
    fn test_sqlite_backend() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.db");
        let jsonl_path = temp.path().join("audit.log");

        // Create database
        let db = Database::open_at(db_path).unwrap();
        let logger = AuditLogger::new(Some(db), jsonl_path.clone(), true);

        assert!(logger.has_database());

        // Log some entries
        logger
            .log_cleaned("test", Path::new("/test/path"), 1024)
            .unwrap();
        logger
            .log_dry_run("test", Path::new("/test/path2"), 2048)
            .unwrap();

        // Read back from SQLite
        let entries = logger.read_recent(10).unwrap();
        assert_eq!(entries.len(), 2);

        // JSONL file should NOT exist (using SQLite)
        assert!(!jsonl_path.exists());
    }
}
