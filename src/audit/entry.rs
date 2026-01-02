//! Audit log entry types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// Type of audit operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditOperation {
    /// Scanning for cleanable items.
    Scan,
    /// Moving item to trash.
    MoveToTrash,
    /// Skipping an item.
    Skip,
    /// Error occurred.
    Error,
}

/// A single audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique identifier for this entry.
    pub id: Uuid,
    /// Timestamp of the operation.
    pub timestamp: DateTime<Utc>,
    /// Type of operation.
    pub operation: AuditOperation,
    /// Cleaner that performed the operation.
    pub cleaner_id: String,
    /// Path that was affected.
    pub path: PathBuf,
    /// Size of the item in bytes.
    pub size: u64,
    /// Human-readable size.
    pub size_human: String,
    /// Whether this was a dry-run.
    pub dry_run: bool,
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Additional context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

impl AuditEntry {
    /// Create a new audit entry for a successful clean operation.
    pub fn cleaned(cleaner_id: &str, path: PathBuf, size: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            operation: AuditOperation::MoveToTrash,
            cleaner_id: cleaner_id.to_string(),
            path,
            size,
            size_human: format_size(size),
            dry_run: false,
            success: true,
            error: None,
            context: None,
        }
    }

    /// Create a new audit entry for a dry-run operation.
    pub fn dry_run(cleaner_id: &str, path: PathBuf, size: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            operation: AuditOperation::MoveToTrash,
            cleaner_id: cleaner_id.to_string(),
            path,
            size,
            size_human: format_size(size),
            dry_run: true,
            success: true,
            error: None,
            context: Some("DRY RUN - no actual deletion".to_string()),
        }
    }

    /// Create a new audit entry for an error.
    pub fn error(cleaner_id: &str, path: PathBuf, error: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            operation: AuditOperation::Error,
            cleaner_id: cleaner_id.to_string(),
            path,
            size: 0,
            size_human: "0 B".to_string(),
            dry_run: false,
            success: false,
            error: Some(error.to_string()),
            context: None,
        }
    }

    /// Create a new audit entry for a skipped item.
    pub fn skipped(cleaner_id: &str, path: PathBuf, reason: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            operation: AuditOperation::Skip,
            cleaner_id: cleaner_id.to_string(),
            path,
            size: 0,
            size_human: "0 B".to_string(),
            dry_run: false,
            success: true,
            error: None,
            context: Some(reason.to_string()),
        }
    }

    /// Create a new audit entry for a scan operation.
    pub fn scan(cleaner_id: &str, path: PathBuf, size: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            operation: AuditOperation::Scan,
            cleaner_id: cleaner_id.to_string(),
            path,
            size,
            size_human: format_size(size),
            dry_run: false,
            success: true,
            error: None,
            context: None,
        }
    }

    /// Add context to this entry.
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
}

/// Format bytes as human-readable size.
fn format_size(bytes: u64) -> String {
    bytesize::ByteSize(bytes).to_string()
}
