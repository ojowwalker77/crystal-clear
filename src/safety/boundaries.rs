//! Safe boundary definitions for cleaners.
//!
//! Each cleaner operates within a defined boundary and cannot
//! access or modify anything outside that boundary.

use std::fmt;
use std::path::{Path, PathBuf};

use crate::error::CleanmacError;

/// Defines the safe boundary within which a cleaner can operate.
#[derive(Debug, Clone)]
pub struct SafeBoundary {
    /// The root path(s) that define the boundary.
    roots: Vec<PathBuf>,
    /// Maximum depth from root (None = unlimited).
    max_depth: Option<usize>,
    /// Human-readable description for error messages.
    description: String,
}

impl SafeBoundary {
    /// Create a new safe boundary with the given roots.
    pub fn new(roots: Vec<PathBuf>, description: impl Into<String>) -> Self {
        Self {
            roots,
            max_depth: None,
            description: description.into(),
        }
    }

    /// Set a maximum depth limit for this boundary.
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Get the root paths of this boundary.
    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    /// Check if a path is within the safe boundary.
    ///
    /// Returns `Ok(true)` if the path is within bounds.
    /// Returns `Ok(false)` if the path is outside bounds.
    /// Returns `Err` if the path cannot be resolved.
    pub fn contains(&self, path: &Path) -> Result<bool, CleanmacError> {
        // Canonicalize the path to resolve symlinks
        let canonical = path.canonicalize().map_err(|e| CleanmacError::Filesystem {
            path: path.to_path_buf(),
            source: e,
        })?;

        for root in &self.roots {
            // Try to canonicalize the root, skip if it doesn't exist
            let root_canonical = match root.canonicalize() {
                Ok(r) => r,
                Err(_) => continue, // Root doesn't exist, skip
            };

            if canonical.starts_with(&root_canonical) {
                // Check depth if specified
                if let Some(max_depth) = self.max_depth {
                    let relative = canonical.strip_prefix(&root_canonical).map_err(|_| {
                        CleanmacError::Filesystem {
                            path: path.to_path_buf(),
                            source: std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "Failed to compute relative path",
                            ),
                        }
                    })?;
                    let depth = relative.components().count();
                    if depth > max_depth {
                        return Ok(false);
                    }
                }
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Validate that a path is within bounds, returning an error if not.
    pub fn validate(&self, path: &Path) -> Result<(), CleanmacError> {
        let canonical = path.canonicalize().map_err(|e| CleanmacError::Filesystem {
            path: path.to_path_buf(),
            source: e,
        })?;

        if !self.contains(path)? {
            return Err(CleanmacError::BoundaryEscape {
                path: path.to_path_buf(),
                resolved: canonical,
            });
        }

        Ok(())
    }
}

impl fmt::Display for SafeBoundary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} [", self.description)?;
        for (i, root) in self.roots.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", root.display())?;
        }
        write!(f, "]")?;
        if let Some(depth) = self.max_depth {
            write!(f, " (max depth: {})", depth)?;
        }
        Ok(())
    }
}

/// Predefined boundaries for common cleaning operations.
pub mod presets {
    use super::*;

    /// Create a boundary for user cache directories.
    pub fn user_caches() -> SafeBoundary {
        let home = dirs::home_dir().expect("Home directory required");
        SafeBoundary::new(vec![home.join("Library/Caches")], "User cache directories")
    }

    /// Create a boundary for user log directories.
    pub fn user_logs() -> SafeBoundary {
        let home = dirs::home_dir().expect("Home directory required");
        SafeBoundary::new(vec![home.join("Library/Logs")], "User log directories")
    }

    /// Create a boundary for the user's trash.
    pub fn user_trash() -> SafeBoundary {
        let home = dirs::home_dir().expect("Home directory required");
        SafeBoundary::new(vec![home.join(".Trash")], "User trash")
    }

    /// Create a boundary for Xcode developer directories.
    pub fn xcode() -> SafeBoundary {
        let home = dirs::home_dir().expect("Home directory required");
        let developer = home.join("Library/Developer/Xcode");
        SafeBoundary::new(
            vec![
                developer.join("DerivedData"),
                developer.join("Archives"),
                developer.join("iOS DeviceSupport"),
                developer.join("watchOS DeviceSupport"),
                developer.join("tvOS DeviceSupport"),
            ],
            "Xcode developer directories",
        )
    }

    /// Create a boundary for Homebrew directories.
    pub fn homebrew() -> SafeBoundary {
        SafeBoundary::new(
            vec![
                PathBuf::from("/opt/homebrew"),
                PathBuf::from("/usr/local/Homebrew"),
                PathBuf::from("/usr/local/Cellar"),
            ],
            "Homebrew directories",
        )
    }

    /// Create a boundary for npm cache.
    pub fn npm() -> SafeBoundary {
        let home = dirs::home_dir().expect("Home directory required");
        SafeBoundary::new(vec![home.join(".npm")], "npm cache")
    }

    /// Create a boundary for yarn cache.
    pub fn yarn() -> SafeBoundary {
        let home = dirs::home_dir().expect("Home directory required");
        SafeBoundary::new(
            vec![home.join(".yarn/cache"), home.join("Library/Caches/Yarn")],
            "yarn cache",
        )
    }

    /// Create a boundary for Cargo cache.
    pub fn cargo() -> SafeBoundary {
        let home = dirs::home_dir().expect("Home directory required");
        SafeBoundary::new(
            vec![
                home.join(".cargo/registry/cache"),
                home.join(".cargo/registry/src"),
            ],
            "Cargo cache",
        )
    }

    /// Create a boundary for pip cache.
    pub fn pip() -> SafeBoundary {
        let home = dirs::home_dir().expect("Home directory required");
        SafeBoundary::new(
            vec![home.join("Library/Caches/pip"), home.join(".cache/pip")],
            "pip cache",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_boundary_contains_direct_child() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();
        let child = root.join("child");
        fs::create_dir(&child).unwrap();

        let boundary = SafeBoundary::new(vec![root], "test");
        assert!(boundary.contains(&child).unwrap());
    }

    #[test]
    fn test_boundary_rejects_outside_path() {
        let temp1 = TempDir::new().unwrap();
        let temp2 = TempDir::new().unwrap();

        let boundary = SafeBoundary::new(vec![temp1.path().to_path_buf()], "test");
        assert!(!boundary.contains(temp2.path()).unwrap());
    }

    #[test]
    fn test_boundary_depth_limit() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();
        let level1 = root.join("level1");
        let level2 = level1.join("level2");
        let level3 = level2.join("level3");

        fs::create_dir_all(&level3).unwrap();

        let boundary = SafeBoundary::new(vec![root], "test").with_max_depth(2);

        assert!(boundary.contains(&level1).unwrap());
        assert!(boundary.contains(&level2).unwrap());
        assert!(!boundary.contains(&level3).unwrap()); // Exceeds depth
    }

    #[test]
    fn test_symlink_escape_detected() {
        let temp = TempDir::new().unwrap();
        let safe_dir = temp.path().join("safe");
        fs::create_dir(&safe_dir).unwrap();

        // Create a symlink that points outside the boundary
        let escape_link = safe_dir.join("escape");
        #[cfg(unix)]
        std::os::unix::fs::symlink("/tmp", &escape_link).unwrap();

        let boundary = SafeBoundary::new(vec![safe_dir], "test");

        // The symlink target (/tmp) should be detected as outside bounds
        #[cfg(unix)]
        assert!(!boundary.contains(&escape_link).unwrap());
    }
}
