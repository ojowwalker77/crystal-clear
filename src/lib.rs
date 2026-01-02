//! Crystal Clear - A safe cross-platform system cleaner.
//!
//! This library provides the core functionality for the Crystal Clear TUI tool.
//! It is designed with safety as the primary concern:
//!
//! - Files are ONLY moved to trash, never permanently deleted
//! - Protected system paths are hardcoded and cannot be overridden
//! - All operations are logged to an audit file
//! - Dry-run is the default mode
//!
//! ## Platform Support
//!
//! Crystal Clear supports macOS, Linux, and Windows:
//! - Uses the `trash` crate for cross-platform trash operations
//! - Platform-specific cleaners (Xcode on macOS, etc.) are conditionally compiled
//! - Protected paths are defined per-platform

// Setup UniFFI scaffolding first - this MUST come before the ffi module
uniffi::setup_scaffolding!();

pub mod audit;
pub mod cleaners;
pub mod config;
pub mod db;
pub mod discovery;
pub mod disk;
pub mod error;
pub mod ffi;
pub mod platform;
pub mod safety;
pub mod scanner;
pub mod trash;
pub mod tui;

// Re-export commonly used types
pub use config::Config;
pub use db::Database;
pub use error::{exit_codes, CleanmacError};
pub use platform::PlatformPaths;
pub use tui::App;
