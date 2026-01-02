//! Database query implementations.

use std::path::Path;

use chrono::{Duration, Utc};
use rusqlite::{params, Row};

use crate::audit::entry::{AuditEntry, AuditOperation};
use crate::error::CleanmacError;

use super::{Database, DiscoveredPath, DiscoveryPattern, PathCategory, PatternType, ScanType};

impl Database {
    // ==================== Audit Log ====================

    /// Insert an audit entry.
    pub fn insert_audit_entry(&self, entry: &AuditEntry) -> Result<(), CleanmacError> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO audit_log (id, timestamp, operation, cleaner_id, path, size, size_human, dry_run, success, error, context)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    entry.id.to_string(),
                    entry.timestamp.to_rfc3339(),
                    serde_json::to_string(&entry.operation).unwrap_or_default(),
                    entry.cleaner_id,
                    entry.path.to_string_lossy(),
                    entry.size as i64,
                    entry.size_human,
                    entry.dry_run,
                    entry.success,
                    entry.error,
                    entry.context,
                ],
            )?;
            Ok(())
        })
    }

    /// Get recent audit entries.
    pub fn get_recent_audit_entries(&self, limit: usize) -> Result<Vec<AuditEntry>, CleanmacError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, timestamp, operation, cleaner_id, path, size, size_human, dry_run, success, error, context
                 FROM audit_log
                 ORDER BY timestamp DESC
                 LIMIT ?",
            )?;

            let entries = stmt
                .query_map([limit as i64], |row| Self::row_to_audit_entry(row))?
                .filter_map(|r| r.ok())
                .collect();

            Ok(entries)
        })
    }

    /// Delete audit entries older than N days.
    pub fn cleanup_old_audit_entries(&self, days: u32) -> Result<usize, CleanmacError> {
        let cutoff = Utc::now() - Duration::days(days as i64);
        self.with_conn(|conn| {
            let deleted = conn.execute(
                "DELETE FROM audit_log WHERE timestamp < ?",
                [cutoff.to_rfc3339()],
            )?;
            Ok(deleted)
        })
    }

    fn row_to_audit_entry(row: &Row) -> Result<AuditEntry, rusqlite::Error> {
        use std::path::PathBuf;
        use uuid::Uuid;

        let id_str: String = row.get(0)?;
        let timestamp_str: String = row.get(1)?;
        let operation_str: String = row.get(2)?;

        Ok(AuditEntry {
            id: Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::new_v4()),
            timestamp: chrono::DateTime::parse_from_rfc3339(&timestamp_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            operation: serde_json::from_str(&operation_str).unwrap_or(AuditOperation::Scan),
            cleaner_id: row.get(3)?,
            path: PathBuf::from(row.get::<_, String>(4)?),
            size: row.get::<_, i64>(5)? as u64,
            size_human: row.get(6)?,
            dry_run: row.get(7)?,
            success: row.get(8)?,
            error: row.get(9)?,
            context: row.get(10)?,
        })
    }

    // ==================== Discovered Paths ====================

    /// Insert or update a discovered path.
    pub fn upsert_discovered_path(&self, path: &DiscoveredPath) -> Result<(), CleanmacError> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO discovered_paths (path, category, discovery_method, confidence, last_seen, first_seen, last_size, is_active)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(path) DO UPDATE SET
                    category = excluded.category,
                    last_seen = excluded.last_seen,
                    last_size = excluded.last_size,
                    is_active = 1",
                params![
                    path.path.to_string_lossy(),
                    path.category.as_str(),
                    path.discovery_method,
                    path.confidence,
                    path.last_seen.to_rfc3339(),
                    path.first_seen.to_rfc3339(),
                    path.last_size.map(|s| s as i64),
                    path.is_active,
                ],
            )?;
            Ok(())
        })
    }

    /// Get all active discovered paths.
    pub fn get_active_discovered_paths(&self) -> Result<Vec<DiscoveredPath>, CleanmacError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, path, category, discovery_method, confidence, last_seen, first_seen, last_size, times_cleaned, is_active
                 FROM discovered_paths
                 WHERE is_active = 1
                 ORDER BY category, path",
            )?;

            let paths = stmt
                .query_map([], Self::row_to_discovered_path)?
                .filter_map(|r| r.ok())
                .collect();

            Ok(paths)
        })
    }

    /// Get discovered paths by category.
    pub fn get_paths_by_category(
        &self,
        category: &PathCategory,
    ) -> Result<Vec<DiscoveredPath>, CleanmacError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, path, category, discovery_method, confidence, last_seen, first_seen, last_size, times_cleaned, is_active
                 FROM discovered_paths
                 WHERE category = ? AND is_active = 1",
            )?;

            let paths = stmt
                .query_map([category.as_str()], Self::row_to_discovered_path)?
                .filter_map(|r| r.ok())
                .collect();

            Ok(paths)
        })
    }

    /// Mark a path as inactive (no longer exists).
    pub fn deactivate_path(&self, path: &Path) -> Result<(), CleanmacError> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE discovered_paths SET is_active = 0 WHERE path = ?",
                [path.to_string_lossy()],
            )?;
            Ok(())
        })
    }

    /// Increment the times_cleaned counter for a path.
    pub fn record_path_cleaned(&self, path: &Path) -> Result<(), CleanmacError> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE discovered_paths SET times_cleaned = times_cleaned + 1 WHERE path = ?",
                [path.to_string_lossy()],
            )?;
            Ok(())
        })
    }

    fn row_to_discovered_path(row: &Row) -> Result<DiscoveredPath, rusqlite::Error> {
        use std::path::PathBuf;

        Ok(DiscoveredPath {
            id: Some(row.get(0)?),
            path: PathBuf::from(row.get::<_, String>(1)?),
            category: PathCategory::from_str(&row.get::<_, String>(2)?),
            discovery_method: row.get(3)?,
            confidence: row.get(4)?,
            last_seen: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            first_seen: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            last_size: row.get::<_, Option<i64>>(7)?.map(|s| s as u64),
            times_cleaned: row.get::<_, i32>(8)? as u32,
            is_active: row.get(9)?,
        })
    }

    // ==================== Discovery Patterns ====================

    /// Get all enabled discovery patterns.
    pub fn get_discovery_patterns(&self) -> Result<Vec<DiscoveryPattern>, CleanmacError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, pattern, category, pattern_type, priority, enabled, description
                 FROM discovery_patterns
                 WHERE enabled = 1
                 ORDER BY priority DESC",
            )?;

            let patterns = stmt
                .query_map([], |row| {
                    Ok(DiscoveryPattern {
                        id: Some(row.get(0)?),
                        pattern: row.get(1)?,
                        category: PathCategory::from_str(&row.get::<_, String>(2)?),
                        pattern_type: PatternType::from_str(&row.get::<_, String>(3)?),
                        priority: row.get(4)?,
                        enabled: row.get(5)?,
                        description: row.get(6)?,
                    })
                })?
                .filter_map(|r| r.ok())
                .collect();

            Ok(patterns)
        })
    }

    // ==================== Scan History ====================

    /// Record a scan start.
    pub fn start_scan(&self, scan_id: &str, scan_type: ScanType) -> Result<(), CleanmacError> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO scan_history (scan_id, started_at, scan_type)
                 VALUES (?, ?, ?)",
                params![scan_id, Utc::now().to_rfc3339(), scan_type.as_str(),],
            )?;
            Ok(())
        })
    }

    /// Complete a scan record.
    pub fn complete_scan(
        &self,
        scan_id: &str,
        total_items: u32,
        total_size: u64,
        duration_ms: u64,
    ) -> Result<(), CleanmacError> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE scan_history SET
                    completed_at = ?,
                    total_items = ?,
                    total_size = ?,
                    duration_ms = ?
                 WHERE scan_id = ?",
                params![
                    Utc::now().to_rfc3339(),
                    total_items as i64,
                    total_size as i64,
                    duration_ms as i64,
                    scan_id,
                ],
            )?;
            Ok(())
        })
    }

    /// Get the last successful scan timestamp.
    pub fn get_last_scan_time(&self) -> Result<Option<chrono::DateTime<Utc>>, CleanmacError> {
        self.with_conn(|conn| {
            let result: Option<String> = conn
                .query_row(
                    "SELECT completed_at FROM scan_history
                 WHERE completed_at IS NOT NULL
                 ORDER BY completed_at DESC
                 LIMIT 1",
                    [],
                    |row| row.get(0),
                )
                .ok();

            Ok(result.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok()
            }))
        })
    }
}
