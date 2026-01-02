//! Platform abstraction layer for cross-platform support.
//!
//! This module provides platform-specific implementations for:
//! - File system paths (caches, logs, app support directories)
//! - Disk information utilities
//! - Application detection

pub mod paths;
pub mod disk;

pub use paths::PlatformPaths;
pub use disk::DiskInfoProvider;
