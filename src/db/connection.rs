//! Database connection and initialization.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::error::CleanmacError;

use super::migrations;

/// Thread-safe database handle.
#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
    path: PathBuf,
}

impl std::fmt::Debug for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Database")
            .field("path", &self.path)
            .finish()
    }
}

impl Database {
    /// Create or open database at the default location.
    pub fn open() -> Result<Self, CleanmacError> {
        let db_path = Self::default_path()?;
        Self::open_at(db_path)
    }

    /// Get default database path (~/.local/share/crystal-clear/crystal.db).
    pub fn default_path() -> Result<PathBuf, CleanmacError> {
        let data_dir = dirs::data_local_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join(".local/share")))
            .ok_or_else(|| CleanmacError::Config {
                message: "Cannot determine data directory".to_string(),
            })?;

        let app_dir = data_dir.join("crystal-clear");

        // Ensure directory exists
        std::fs::create_dir_all(&app_dir).map_err(|e| CleanmacError::Filesystem {
            path: app_dir.clone(),
            source: e,
        })?;

        Ok(app_dir.join("crystal.db"))
    }

    /// Open database at specific path.
    pub fn open_at(path: PathBuf) -> Result<Self, CleanmacError> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| CleanmacError::Filesystem {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        let conn = Connection::open(&path).map_err(|e| CleanmacError::Config {
            message: format!("Failed to open database: {}", e),
        })?;

        // Enable WAL mode for better concurrency
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| CleanmacError::Config {
                message: format!("Failed to configure database: {}", e),
            })?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            path,
        };

        // Run migrations
        migrations::run(&db)?;

        Ok(db)
    }

    /// Execute a function with the connection.
    pub fn with_conn<F, T>(&self, f: F) -> Result<T, CleanmacError>
    where
        F: FnOnce(&Connection) -> Result<T, rusqlite::Error>,
    {
        let conn = self.conn.lock().map_err(|_| CleanmacError::Config {
            message: "Database lock poisoned".to_string(),
        })?;

        f(&conn).map_err(|e| CleanmacError::Config {
            message: format!("Database error: {}", e),
        })
    }

    /// Execute a mutable function with the connection.
    pub fn with_conn_mut<F, T>(&self, f: F) -> Result<T, CleanmacError>
    where
        F: FnOnce(&mut Connection) -> Result<T, rusqlite::Error>,
    {
        let mut conn = self.conn.lock().map_err(|_| CleanmacError::Config {
            message: "Database lock poisoned".to_string(),
        })?;

        f(&mut conn).map_err(|e| CleanmacError::Config {
            message: format!("Database error: {}", e),
        })
    }

    /// Get the database path.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}
