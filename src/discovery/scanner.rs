//! Disk scanning for discoverable cleanable paths.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use chrono::Utc;
use walkdir::WalkDir;

use crate::db::{Database, DiscoveredPath, ScanType};
use crate::error::CleanmacError;
use crate::safety::is_protected;
use crate::scanner::calculate_size;

use super::heuristics;

/// Configuration for path discovery.
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Maximum depth to scan from root directories.
    pub max_depth: usize,
    /// Minimum directory size (bytes) to consider.
    pub min_size: u64,
    /// Directories to skip entirely.
    pub skip_dirs: HashSet<String>,
    /// Use cached paths if available and recent.
    pub use_cache: bool,
    /// Cache freshness threshold in hours.
    pub cache_max_age_hours: u32,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        let mut skip_dirs = HashSet::new();
        // Directories that are too slow or pointless to scan
        skip_dirs.insert(".git".to_string());
        skip_dirs.insert(".svn".to_string());
        skip_dirs.insert(".hg".to_string());
        skip_dirs.insert(".Trash".to_string());
        skip_dirs.insert("Photos Library.photoslibrary".to_string());
        skip_dirs.insert("Music".to_string());
        skip_dirs.insert("Movies".to_string());

        Self {
            max_depth: 5,
            min_size: 1024 * 1024, // 1MB minimum
            skip_dirs,
            use_cache: true,
            cache_max_age_hours: 24,
        }
    }
}

/// Path discovery engine.
pub struct PathDiscovery {
    db: Database,
    config: DiscoveryConfig,
}

impl PathDiscovery {
    /// Create a new path discovery engine.
    pub fn new(db: Database, config: DiscoveryConfig) -> Self {
        Self { db, config }
    }

    /// Create with default configuration.
    pub fn with_defaults(db: Database) -> Self {
        Self::new(db, DiscoveryConfig::default())
    }

    /// Get cleanable paths, using cache if available or performing discovery.
    pub fn get_cleanable_paths(&self) -> Result<Vec<DiscoveredPath>, CleanmacError> {
        if self.config.use_cache {
            if let Some(last_scan) = self.db.get_last_scan_time()? {
                let age_hours = (Utc::now() - last_scan).num_hours();
                if age_hours < self.config.cache_max_age_hours as i64 {
                    // Use cached paths, but verify they still exist
                    return self.get_verified_cached_paths();
                }
            }
        }

        // No cache available, return empty (user should trigger discovery manually)
        self.get_verified_cached_paths()
    }

    /// Get cached paths, marking missing ones as inactive.
    fn get_verified_cached_paths(&self) -> Result<Vec<DiscoveredPath>, CleanmacError> {
        let cached = self.db.get_active_discovered_paths()?;
        let mut valid = Vec::with_capacity(cached.len());

        for path in cached {
            if path.path.exists() {
                valid.push(path);
            } else {
                self.db.deactivate_path(&path.path)?;
            }
        }

        Ok(valid)
    }

    /// Perform full path discovery.
    /// This scans the filesystem to find cleanable directories.
    pub fn discover_all(
        &self,
        progress_callback: Option<&dyn Fn(&str)>,
    ) -> Result<Vec<DiscoveredPath>, CleanmacError> {
        let scan_id = uuid::Uuid::new_v4().to_string();
        self.db.start_scan(&scan_id, ScanType::Full)?;
        let start = Instant::now();

        let mut discovered = Vec::new();

        // Scan user Library directories
        if let Some(home) = dirs::home_dir() {
            let library = home.join("Library");

            if let Some(cb) = progress_callback {
                cb("Scanning ~/Library/Caches...");
            }
            discovered.extend(self.scan_directory(&library.join("Caches"))?);

            if let Some(cb) = progress_callback {
                cb("Scanning ~/Library/Logs...");
            }
            discovered.extend(self.scan_directory(&library.join("Logs"))?);

            if let Some(cb) = progress_callback {
                cb("Scanning ~/Library/Developer...");
            }
            discovered.extend(self.scan_directory(&library.join("Developer"))?);

            // Scan common developer directories
            if let Some(cb) = progress_callback {
                cb("Scanning ~/.cargo...");
            }
            discovered.extend(self.scan_directory(&home.join(".cargo"))?);

            if let Some(cb) = progress_callback {
                cb("Scanning ~/.npm...");
            }
            discovered.extend(self.scan_directory(&home.join(".npm"))?);

            if let Some(cb) = progress_callback {
                cb("Scanning ~/.yarn...");
            }
            discovered.extend(self.scan_directory(&home.join(".yarn"))?);

            if let Some(cb) = progress_callback {
                cb("Scanning ~/.cache...");
            }
            discovered.extend(self.scan_directory(&home.join(".cache"))?);
        }

        // Scan /opt/homebrew if present
        let homebrew = PathBuf::from("/opt/homebrew");
        if homebrew.exists() {
            if let Some(cb) = progress_callback {
                cb("Scanning /opt/homebrew...");
            }
            discovered.extend(self.scan_directory(&homebrew)?);
        }

        if let Some(cb) = progress_callback {
            cb("Saving discovered paths...");
        }

        // Save discovered paths to database
        for path in &discovered {
            self.db.upsert_discovered_path(path)?;
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        let total_size: u64 = discovered.iter().filter_map(|p| p.last_size).sum();
        self.db
            .complete_scan(&scan_id, discovered.len() as u32, total_size, duration_ms)?;

        Ok(discovered)
    }

    /// Perform incremental discovery (only update existing cached paths).
    pub fn discover_incremental(&self) -> Result<Vec<DiscoveredPath>, CleanmacError> {
        let scan_id = uuid::Uuid::new_v4().to_string();
        self.db.start_scan(&scan_id, ScanType::Incremental)?;
        let start = Instant::now();

        // Get existing cached paths
        let mut existing = self.db.get_active_discovered_paths()?;

        // Verify existing paths still exist and update sizes
        for path in &mut existing {
            if path.path.exists() {
                path.last_size = Some(calculate_size(&path.path));
                path.last_seen = Utc::now();
                self.db.upsert_discovered_path(path)?;
            } else {
                self.db.deactivate_path(&path.path)?;
            }
        }

        existing.retain(|p| p.is_active && p.path.exists());

        let duration_ms = start.elapsed().as_millis() as u64;
        let total_size: u64 = existing.iter().filter_map(|p| p.last_size).sum();
        self.db
            .complete_scan(&scan_id, existing.len() as u32, total_size, duration_ms)?;

        Ok(existing)
    }

    /// Scan a directory tree for cleanable paths.
    fn scan_directory(&self, root: &Path) -> Result<Vec<DiscoveredPath>, CleanmacError> {
        let mut discovered = Vec::new();

        if !root.exists() {
            return Ok(discovered);
        }

        let walker = WalkDir::new(root)
            .max_depth(self.config.max_depth)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| self.should_process_entry(e));

        for entry in walker.filter_map(|e| e.ok()) {
            let path = entry.path();

            // Skip non-directories
            if !path.is_dir() {
                continue;
            }

            // Skip protected paths
            if is_protected(path) {
                continue;
            }

            // Check against heuristics
            if let Some((category, confidence)) = heuristics::detect_cleanable_directory(path) {
                let size = calculate_size(path);

                if size >= self.config.min_size {
                    discovered.push(DiscoveredPath {
                        id: None,
                        path: path.to_path_buf(),
                        category,
                        discovery_method: "heuristic".to_string(),
                        confidence,
                        last_seen: Utc::now(),
                        first_seen: Utc::now(),
                        last_size: Some(size),
                        times_cleaned: 0,
                        is_active: true,
                    });
                }
            }
        }

        Ok(discovered)
    }

    /// Check if an entry should be processed.
    fn should_process_entry(&self, entry: &walkdir::DirEntry) -> bool {
        if let Some(name) = entry.file_name().to_str() {
            // Skip hidden directories (except specific ones we want)
            if name.starts_with('.') && !self.is_interesting_hidden_dir(name) {
                return false;
            }

            // Skip explicitly excluded directories
            if self.config.skip_dirs.contains(name) {
                return false;
            }
        }

        true
    }

    /// Check if a hidden directory is interesting to scan.
    fn is_interesting_hidden_dir(&self, name: &str) -> bool {
        matches!(name, ".cargo" | ".npm" | ".yarn" | ".cache")
    }

    /// Get database reference.
    pub fn database(&self) -> &Database {
        &self.db
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_discovery_config_defaults() {
        let config = DiscoveryConfig::default();
        assert_eq!(config.max_depth, 5);
        assert!(config.skip_dirs.contains(".git"));
    }

    #[test]
    fn test_scanner_creation() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.db");
        let db = Database::open_at(db_path).unwrap();

        let scanner = PathDiscovery::with_defaults(db);
        assert!(scanner.get_cleanable_paths().is_ok());
    }
}
