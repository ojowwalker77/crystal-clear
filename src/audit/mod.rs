//! Audit logging module for CleanMac.
//!
//! All cleaning operations are logged to an audit file in JSONL format.
//! This provides a complete history of what was cleaned and when.

pub mod entry;
pub mod logger;

pub use entry::{AuditEntry, AuditOperation};
pub use logger::{default_audit_path, AuditLogger};
