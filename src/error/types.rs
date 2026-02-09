//! Custom error types for CleanMac.
//!
//! Errors are categorized by severity:
//! - Fatal: Safety violations that immediately terminate the program
//! - Recoverable: Errors that are logged but allow processing to continue

use std::path::PathBuf;
use thiserror::Error;

/// Main error type for CleanMac operations.
#[derive(Error, Debug)]
pub enum CleanmacError {
    // ============================================================
    // FATAL ERRORS - These immediately terminate the program
    // ============================================================
    /// Attempted to access a protected system path.
    /// This is a critical safety violation.
    #[error("SAFETY VIOLATION: '{path}' is a protected system path and cannot be modified")]
    ProtectedPath { path: PathBuf },

    /// Path resolves to a location outside the allowed safe boundary.
    /// This could indicate a symlink escape attack.
    #[error("SAFETY VIOLATION: '{path}' resolves to '{resolved}' which escapes the safe boundary")]
    BoundaryEscape { path: PathBuf, resolved: PathBuf },

    /// Detected a circular symlink chain.
    #[error("SAFETY VIOLATION: Symlink loop detected at '{path}'")]
    SymlinkLoop { path: PathBuf },

    /// Configuration file contains invalid settings.
    #[error("Configuration error: {message}")]
    Config { message: String },

    /// Invalid path specified in configuration.
    #[error("Invalid path in configuration: '{path}'")]
    InvalidConfigPath { path: PathBuf },

    // ============================================================
    // RECOVERABLE ERRORS - Logged and processing continues
    // ============================================================
    /// Permission denied when accessing a file or directory.
    #[error("Permission denied: '{path}'")]
    PermissionDenied { path: PathBuf },

    /// Path does not exist.
    #[error("Path not found: '{path}'")]
    PathNotFound { path: PathBuf },

    /// General filesystem error.
    #[error("Filesystem error at '{path}': {source}")]
    Filesystem {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to move file to trash.
    #[error("Failed to move to trash: '{path}' - {reason}")]
    TrashFailed { path: PathBuf, reason: String },

    // ============================================================
    // USER INTERACTION ERRORS
    // ============================================================
    /// Operation exceeds size limit and requires --force flag.
    #[error("Size limit exceeded: {size} bytes exceeds {limit} bytes. Use --force to proceed.")]
    SizeLimitExceeded { size: u64, limit: u64 },

    /// User cancelled the operation.
    #[error("Operation cancelled by user")]
    Cancelled,

    // ============================================================
    // CLEANER ERRORS
    // ============================================================
    /// A cleaner failed to complete its operation.
    #[error("Cleaner '{cleaner}' failed: {message}")]
    CleanerFailed { cleaner: String, message: String },

    /// Cleaner is not available on this system.
    #[error("Cleaner '{cleaner}' is not available: {reason}")]
    CleanerUnavailable { cleaner: String, reason: String },
}

impl CleanmacError {
    /// Returns true if this error is a fatal safety violation.
    /// Fatal errors should immediately terminate the program.
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            CleanmacError::ProtectedPath { .. }
                | CleanmacError::BoundaryEscape { .. }
                | CleanmacError::SymlinkLoop { .. }
                | CleanmacError::Config { .. }
                | CleanmacError::InvalidConfigPath { .. }
        )
    }

    /// Returns true if this error can be logged and processing can continue.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            CleanmacError::PermissionDenied { .. }
                | CleanmacError::PathNotFound { .. }
                | CleanmacError::Filesystem { .. }
                | CleanmacError::TrashFailed { .. }
        )
    }

    /// Returns the appropriate exit code for this error.
    pub fn exit_code(&self) -> i32 {
        match self {
            // Safety violations
            CleanmacError::ProtectedPath { .. }
            | CleanmacError::BoundaryEscape { .. }
            | CleanmacError::SymlinkLoop { .. } => 2,

            // Configuration errors
            CleanmacError::Config { .. } | CleanmacError::InvalidConfigPath { .. } => 3,

            // User cancelled
            CleanmacError::Cancelled => 4,

            // All other errors
            _ => 1,
        }
    }
}

/// Exit codes for CleanMac.
pub mod exit_codes {
    /// Operation completed successfully.
    pub const SUCCESS: i32 = 0;
    /// General error.
    pub const ERROR: i32 = 1;
    /// Safety violation.
    pub const SAFETY_VIOLATION: i32 = 2;
    /// Configuration error.
    pub const CONFIG_ERROR: i32 = 3;
    /// User cancelled.
    pub const CANCELLED: i32 = 4;
    /// Partial success (some items failed).
    pub const PARTIAL_SUCCESS: i32 = 5;
}
