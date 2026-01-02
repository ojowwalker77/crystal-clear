//! Path validation module.
//!
//! This module provides comprehensive path validation including:
//! - Protected path checking
//! - Symlink resolution and escape detection
//! - Boundary validation

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::error::CleanmacError;
use crate::safety::boundaries::SafeBoundary;
use crate::safety::protected_paths::{self, get_protected_paths};

/// Result of path validation.
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    /// Path is safe to clean.
    Safe,
    /// Path is protected and cannot be cleaned under any circumstances.
    HardProtected { reason: String },
    /// Path is excluded by user configuration.
    UserExcluded,
    /// Path escapes safe boundaries (symlink escape, etc.).
    BoundaryViolation { reason: String },
    /// Path doesn't exist.
    NotFound,
    /// Permission denied.
    PermissionDenied,
}

/// Path validator with configurable exclusions.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PathValidator {
    /// Additional user-configured exclusions.
    exclusions: HashSet<PathBuf>,
    /// Cached protected paths.
    protected_paths: HashSet<PathBuf>,
}

impl PathValidator {
    /// Create a new path validator with the given exclusions.
    pub fn new(exclusions: Vec<PathBuf>) -> Self {
        // Canonicalize exclusions so they match canonical paths during validation
        let exclusions = exclusions
            .into_iter()
            .filter_map(|p| p.canonicalize().ok())
            .collect();
        Self {
            exclusions,
            protected_paths: get_protected_paths(),
        }
    }

    /// Create a validator with no extra exclusions.
    pub fn default_validator() -> Self {
        Self::new(Vec::new())
    }

    /// Validate a path against protected paths and boundaries.
    pub fn validate(&self, path: &Path, boundary: &SafeBoundary) -> ValidationResult {
        // Step 1: Check if path exists
        if !path.exists() {
            return ValidationResult::NotFound;
        }

        // Step 2: Resolve to canonical path (follows symlinks)
        let canonical = match self.resolve_canonical(path) {
            Ok(p) => p,
            Err(_) => {
                // If we can't resolve, check if it's a permission issue
                if let Err(e) = std::fs::metadata(path) {
                    if e.kind() == std::io::ErrorKind::PermissionDenied {
                        return ValidationResult::PermissionDenied;
                    }
                }
                return ValidationResult::NotFound;
            }
        };

        // Step 3: Check hard-protected paths (NEVER bypass)
        if let Some(reason) = self.check_hard_protected(&canonical) {
            return ValidationResult::HardProtected { reason };
        }

        // Step 4: Check if canonical path escapes the safe boundary
        match boundary.contains(&canonical) {
            Ok(true) => {}
            Ok(false) => {
                return ValidationResult::BoundaryViolation {
                    reason: format!(
                        "Path '{}' resolves to '{}' which is outside boundary: {}",
                        path.display(),
                        canonical.display(),
                        boundary
                    ),
                };
            }
            Err(_) => {
                return ValidationResult::BoundaryViolation {
                    reason: format!(
                        "Cannot verify path '{}' is within boundary",
                        path.display()
                    ),
                };
            }
        }

        // Step 5: Check symlink validity
        if path.is_symlink() {
            if let Err(reason) = self.validate_symlink(path) {
                return ValidationResult::BoundaryViolation { reason };
            }
        }

        // Step 6: Check user exclusions
        if self.is_user_excluded(&canonical) {
            return ValidationResult::UserExcluded;
        }

        ValidationResult::Safe
    }

    /// Quick check if a path is protected (without full validation).
    pub fn is_protected(&self, path: &Path) -> bool {
        protected_paths::is_protected(path)
    }

    /// Resolve a path to its canonical form (following symlinks).
    fn resolve_canonical(&self, path: &Path) -> Result<PathBuf, std::io::Error> {
        path.canonicalize()
    }

    /// Check if a path is in the hardcoded protected list.
    fn check_hard_protected(&self, canonical: &Path) -> Option<String> {
        protected_paths::get_protection_reason(canonical)
    }

    /// Check if a path is in the user exclusion list.
    fn is_user_excluded(&self, canonical: &Path) -> bool {
        // Check exact match
        if self.exclusions.contains(canonical) {
            return true;
        }

        // Check if path is under an excluded directory
        for exclusion in &self.exclusions {
            if canonical.starts_with(exclusion) {
                return true;
            }
        }

        false
    }

    /// Validate symlink for loops and escapes.
    fn validate_symlink(&self, path: &Path) -> Result<(), String> {
        let mut visited = HashSet::new();
        let mut current = path.to_path_buf();
        let max_depth = 40; // Reasonable limit to detect loops

        for _ in 0..max_depth {
            if !current.is_symlink() {
                return Ok(());
            }

            // Track the symlink path itself (before resolution) to detect loops
            // Using the path we're about to follow, not its canonical target
            if !visited.insert(current.clone()) {
                return Err(format!(
                    "Symlink loop detected: '{}' revisits '{}'",
                    path.display(),
                    current.display()
                ));
            }

            // Read the symlink target
            let target = std::fs::read_link(&current)
                .map_err(|e| format!("Cannot read symlink: {}", e))?;

            // If relative, resolve relative to the symlink's directory
            current = if target.is_absolute() {
                target
            } else {
                current
                    .parent()
                    .unwrap_or(Path::new("/"))
                    .join(target)
            };
        }

        Err(format!(
            "Symlink chain too deep (>{} levels): '{}'",
            max_depth,
            path.display()
        ))
    }
}

/// Convert ValidationResult to a Result for cleaner error handling.
impl ValidationResult {
    /// Convert to a Result, with Safe becoming Ok(()) and others becoming Err.
    pub fn into_result(self, path: &Path) -> Result<(), CleanmacError> {
        match self {
            ValidationResult::Safe => Ok(()),
            ValidationResult::HardProtected { reason: _ } => {
                Err(CleanmacError::ProtectedPath {
                    path: path.to_path_buf(),
                })
            }
            ValidationResult::UserExcluded => Ok(()), // Excluded paths are silently skipped
            ValidationResult::BoundaryViolation { reason: _ } => {
                Err(CleanmacError::BoundaryEscape {
                    path: path.to_path_buf(),
                    resolved: path.canonicalize().unwrap_or_else(|_| path.to_path_buf()),
                })
            }
            ValidationResult::NotFound => Err(CleanmacError::PathNotFound {
                path: path.to_path_buf(),
            }),
            ValidationResult::PermissionDenied => Err(CleanmacError::PermissionDenied {
                path: path.to_path_buf(),
            }),
        }
    }

    /// Returns true if this result means the path should be skipped (but not an error).
    pub fn should_skip(&self) -> bool {
        matches!(
            self,
            ValidationResult::UserExcluded | ValidationResult::NotFound
        )
    }

    /// Returns true if this result is a blocking error.
    pub fn is_error(&self) -> bool {
        matches!(
            self,
            ValidationResult::HardProtected { .. }
                | ValidationResult::BoundaryViolation { .. }
                | ValidationResult::PermissionDenied
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_system_paths_rejected() {
        let validator = PathValidator::default_validator();
        let _temp = TempDir::new().unwrap();
        let boundary = SafeBoundary::new(vec![PathBuf::from("/")], "test");

        // These should all be protected (platform-specific paths)
        #[cfg(target_os = "macos")]
        let paths = &["/", "/System", "/Applications", "/Library", "/bin", "/usr"];
        #[cfg(target_os = "linux")]
        let paths = &["/", "/usr", "/etc"];  // /bin may be symlink on some distros
        #[cfg(target_os = "windows")]
        let paths = &["C:\\Windows", "C:\\Program Files"];

        for path in paths {
            let path = PathBuf::from(path);
            if path.exists() {
                let result = validator.validate(&path, &boundary);
                assert!(
                    matches!(result, ValidationResult::HardProtected { .. }),
                    "Expected {} to be protected",
                    path.display()
                );
            }
        }
    }

    #[test]
    fn test_user_exclusions() {
        let temp = TempDir::new().unwrap();
        let excluded_dir = temp.path().join("excluded");
        fs::create_dir(&excluded_dir).unwrap();

        let validator = PathValidator::new(vec![excluded_dir.clone()]);
        let boundary = SafeBoundary::new(vec![temp.path().to_path_buf()], "test");

        let result = validator.validate(&excluded_dir, &boundary);
        assert_eq!(result, ValidationResult::UserExcluded);
    }

    #[test]
    fn test_nonexistent_path() {
        let validator = PathValidator::default_validator();
        let temp = TempDir::new().unwrap();
        let boundary = SafeBoundary::new(vec![temp.path().to_path_buf()], "test");

        let nonexistent = temp.path().join("does-not-exist");
        let result = validator.validate(&nonexistent, &boundary);
        assert_eq!(result, ValidationResult::NotFound);
    }

    #[test]
    fn test_safe_path_accepted() {
        let temp = TempDir::new().unwrap();
        let safe_file = temp.path().join("safe-file.txt");
        fs::write(&safe_file, "content").unwrap();

        let validator = PathValidator::default_validator();
        let boundary = SafeBoundary::new(vec![temp.path().to_path_buf()], "test");

        let result = validator.validate(&safe_file, &boundary);
        assert_eq!(result, ValidationResult::Safe);
    }

    #[test]
    #[cfg(unix)]
    fn test_symlink_outside_boundary_rejected() {
        let temp = TempDir::new().unwrap();
        let safe_dir = temp.path().join("safe");
        let outside_dir = temp.path().join("outside");
        fs::create_dir(&safe_dir).unwrap();
        fs::create_dir(&outside_dir).unwrap();

        // Create symlink pointing outside the safe boundary (but still in temp)
        let escape_link = safe_dir.join("escape");
        std::os::unix::fs::symlink(&outside_dir, &escape_link).unwrap();

        let validator = PathValidator::default_validator();
        let boundary = SafeBoundary::new(vec![safe_dir], "test");

        let result = validator.validate(&escape_link, &boundary);
        // Should be rejected - either as boundary violation or hard protected
        assert!(
            matches!(result, ValidationResult::BoundaryViolation { .. })
                || matches!(result, ValidationResult::HardProtected { .. }),
            "Expected symlink escape to be rejected, got {:?}",
            result
        );
    }
}
