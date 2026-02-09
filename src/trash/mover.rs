//! Cross-platform trash mover implementation.
//!
//! This is the ONLY module that performs file deletion operations.
//! Files are moved to the system trash, NEVER permanently deleted.
//!
//! Uses the `trash` crate for cross-platform trash support:
//! - macOS: Uses Finder's trash
//! - Linux: Follows freedesktop.org trash specification
//! - Windows: Uses the Recycle Bin

use std::path::{Path, PathBuf};

use crate::error::CleanmacError;
use crate::safety::{is_protected, PathValidator, SafeBoundary, ValidationResult};

/// Trash mover that safely moves files to the system trash.
#[derive(Debug)]
pub struct TrashMover {
    /// Path validator for safety checks.
    validator: PathValidator,
}

impl TrashMover {
    /// Create a new trash mover.
    pub fn new(exclusions: Vec<PathBuf>) -> Result<Self, CleanmacError> {
        Ok(Self {
            validator: PathValidator::new(exclusions),
        })
    }

    /// Create a trash mover with default settings.
    pub fn default_mover() -> Result<Self, CleanmacError> {
        Self::new(Vec::new())
    }

    /// Move a file or directory to the system trash.
    ///
    /// Returns the canonical path of the item that was trashed.
    pub fn move_to_trash(&self, path: &Path) -> Result<PathBuf, CleanmacError> {
        // CRITICAL: Final safety check before any operation
        if is_protected(path) {
            return Err(CleanmacError::ProtectedPath {
                path: path.to_path_buf(),
            });
        }

        // Canonicalize the path
        let canonical = path.canonicalize().map_err(|e| CleanmacError::Filesystem {
            path: path.to_path_buf(),
            source: e,
        })?;

        // Double-check the canonical path is also not protected
        if is_protected(&canonical) {
            return Err(CleanmacError::ProtectedPath { path: canonical });
        }

        // Use the trash crate for cross-platform trash operations
        trash::delete(&canonical).map_err(|e| CleanmacError::TrashFailed {
            path: path.to_path_buf(),
            reason: format!("Failed to move to trash: {}", e),
        })?;

        // Return the path that was trashed
        Ok(canonical)
    }

    /// Move a file with validation against a boundary.
    pub fn move_to_trash_validated(
        &self,
        path: &Path,
        boundary: &SafeBoundary,
    ) -> Result<PathBuf, CleanmacError> {
        // Validate path before moving
        let result = self.validator.validate(path, boundary);

        match result {
            ValidationResult::Safe => self.move_to_trash(path),
            ValidationResult::HardProtected { reason: _ } => Err(CleanmacError::ProtectedPath {
                path: path.to_path_buf(),
            }),
            ValidationResult::BoundaryViolation { reason: _ } => {
                Err(CleanmacError::BoundaryEscape {
                    path: path.to_path_buf(),
                    resolved: path.canonicalize().unwrap_or_else(|_| path.to_path_buf()),
                })
            }
            ValidationResult::UserExcluded => Err(CleanmacError::TrashFailed {
                path: path.to_path_buf(),
                reason: "Path is excluded by user configuration".to_string(),
            }),
            ValidationResult::NotFound => Err(CleanmacError::PathNotFound {
                path: path.to_path_buf(),
            }),
            ValidationResult::PermissionDenied => Err(CleanmacError::PermissionDenied {
                path: path.to_path_buf(),
            }),
        }
    }

    /// Check if trash operations are supported on this platform.
    pub fn is_supported() -> bool {
        // The trash crate supports all three major platforms
        cfg!(any(
            target_os = "macos",
            target_os = "linux",
            target_os = "windows"
        ))
    }

    /// Verify that trash operations work correctly.
    /// This creates a temporary file and trashes it as a test.
    pub fn verify_writable(&self) -> Result<(), CleanmacError> {
        use std::fs;

        // Create a temp file in the system temp directory
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join(".crystal_trash_test");

        // Write test file
        fs::write(&test_file, "trash test").map_err(|e| CleanmacError::TrashFailed {
            path: temp_dir.clone(),
            reason: format!("Cannot create test file: {}", e),
        })?;

        // Try to trash it
        trash::delete(&test_file).map_err(|e| CleanmacError::TrashFailed {
            path: test_file.clone(),
            reason: format!("Cannot move to trash: {}", e),
        })?;

        Ok(())
    }

    /// Get a reference to the path validator.
    pub fn validator(&self) -> &PathValidator {
        &self.validator
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_mover() -> TrashMover {
        TrashMover {
            validator: PathValidator::default_validator(),
        }
    }

    #[test]
    fn test_move_file_to_trash() {
        let temp = TempDir::new().unwrap();
        let mover = create_test_mover();

        // Create a test file
        let test_file = temp.path().join("test.txt");
        fs::write(&test_file, "content").unwrap();

        // Move to trash
        let result = mover.move_to_trash(&test_file);

        // Should succeed (trash crate handles the destination)
        assert!(result.is_ok());

        // Original should be gone
        assert!(!test_file.exists());
    }

    #[test]
    fn test_move_directory_to_trash() {
        let temp = TempDir::new().unwrap();
        let mover = create_test_mover();

        // Create a test directory
        let test_dir = temp.path().join("testdir");
        fs::create_dir(&test_dir).unwrap();
        fs::write(test_dir.join("file.txt"), "content").unwrap();

        // Move to trash
        let result = mover.move_to_trash(&test_dir);

        // Should succeed
        assert!(result.is_ok());

        // Original should be gone
        assert!(!test_dir.exists());
    }

    #[test]
    fn test_protected_path_rejected() {
        let mover = create_test_mover();

        // Try to move a system path - protection check should fire
        #[cfg(target_os = "macos")]
        let result = mover.move_to_trash(Path::new("/System"));
        #[cfg(target_os = "linux")]
        let result = mover.move_to_trash(Path::new("/usr"));
        #[cfg(target_os = "windows")]
        let result = mover.move_to_trash(Path::new("C:\\Windows"));

        assert!(result.is_err());
    }

    #[test]
    fn test_is_supported() {
        // Should be supported on all major platforms
        assert!(TrashMover::is_supported());
    }
}
