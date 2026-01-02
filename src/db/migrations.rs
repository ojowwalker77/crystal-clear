//! Database schema migrations.

use crate::error::CleanmacError;

use super::Database;

const SCHEMA_VERSION: i32 = 1;

/// Run all pending migrations.
pub fn run(db: &Database) -> Result<(), CleanmacError> {
    db.with_conn_mut(|conn| {
        // Create schema version table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY
            )",
            [],
        )?;

        let current_version: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if current_version < 1 {
            migrate_v1(conn)?;
        }

        Ok(())
    })
}

fn migrate_v1(conn: &mut rusqlite::Connection) -> Result<(), rusqlite::Error> {
    let tx = conn.transaction()?;

    // Audit log table (replaces JSONL)
    tx.execute(
        "CREATE TABLE IF NOT EXISTS audit_log (
            id TEXT PRIMARY KEY,
            timestamp TEXT NOT NULL,
            operation TEXT NOT NULL,
            cleaner_id TEXT NOT NULL,
            path TEXT NOT NULL,
            size INTEGER NOT NULL,
            size_human TEXT NOT NULL,
            dry_run INTEGER NOT NULL,
            success INTEGER NOT NULL,
            error TEXT,
            context TEXT,
            created_at TEXT DEFAULT (datetime('now'))
        )",
        [],
    )?;
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp)",
        [],
    )?;
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_audit_cleaner ON audit_log(cleaner_id)",
        [],
    )?;

    // Discovered paths cache
    tx.execute(
        "CREATE TABLE IF NOT EXISTS discovered_paths (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL UNIQUE,
            category TEXT NOT NULL,
            discovery_method TEXT NOT NULL,
            confidence REAL NOT NULL DEFAULT 1.0,
            last_seen TEXT NOT NULL,
            first_seen TEXT NOT NULL,
            last_size INTEGER,
            times_cleaned INTEGER DEFAULT 0,
            is_active INTEGER DEFAULT 1,
            metadata TEXT
        )",
        [],
    )?;
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_paths_category ON discovered_paths(category)",
        [],
    )?;
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_paths_active ON discovered_paths(is_active)",
        [],
    )?;

    // Scan history / metadata
    tx.execute(
        "CREATE TABLE IF NOT EXISTS scan_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            scan_id TEXT NOT NULL UNIQUE,
            started_at TEXT NOT NULL,
            completed_at TEXT,
            scan_type TEXT NOT NULL,
            total_items INTEGER,
            total_size INTEGER,
            items_by_category TEXT,
            duration_ms INTEGER,
            error TEXT
        )",
        [],
    )?;

    // Discovery patterns (for dynamic path detection)
    tx.execute(
        "CREATE TABLE IF NOT EXISTS discovery_patterns (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            pattern TEXT NOT NULL,
            category TEXT NOT NULL,
            pattern_type TEXT NOT NULL,
            priority INTEGER DEFAULT 50,
            enabled INTEGER DEFAULT 1,
            description TEXT
        )",
        [],
    )?;

    // Insert default discovery patterns
    tx.execute_batch(
        r#"
        INSERT OR IGNORE INTO discovery_patterns (pattern, category, pattern_type, priority, description) VALUES
        ('Cache', 'cache', 'name_contains', 50, 'Directories containing Cache in name'),
        ('Caches', 'cache', 'name_contains', 60, 'Standard Caches directories'),
        ('cache', 'cache', 'name_contains', 40, 'Lowercase cache directories'),
        ('_cacache', 'cache', 'name_contains', 70, 'npm-style cacache'),
        ('Logs', 'logs', 'name_contains', 60, 'Standard Logs directories'),
        ('logs', 'logs', 'name_contains', 40, 'Lowercase logs directories'),
        ('tmp', 'temp', 'name_contains', 50, 'Temporary directories'),
        ('temp', 'temp', 'name_contains', 50, 'Temporary directories'),
        ('Temp', 'temp', 'name_contains', 50, 'Temporary directories'),
        ('DerivedData', 'build', 'name_contains', 80, 'Xcode derived data'),
        ('node_modules', 'deps', 'name_contains', 70, 'Node.js dependencies'),
        ('target', 'build', 'name_contains', 60, 'Rust/Maven build output'),
        ('build', 'build', 'name_contains', 40, 'Generic build directories'),
        ('dist', 'build', 'name_contains', 40, 'Distribution directories');
        "#,
    )?;

    tx.execute("INSERT INTO schema_version (version) VALUES (?)", [SCHEMA_VERSION])?;

    tx.commit()?;
    Ok(())
}
