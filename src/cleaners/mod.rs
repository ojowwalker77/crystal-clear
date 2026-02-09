//! Cleaners module - all cleaning implementations.

pub mod apps;
pub mod base;
pub mod developer;
pub mod dynamic;
pub mod system;
pub mod traits;

pub use traits::{
    CleanResult, CleanableItem, Cleaner, CleanerCategory, CleanerContext, ItemType, RiskLevel,
    ScanResult,
};

// Re-export individual cleaners
pub use apps::AppsLeftoversCleaner;
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub use developer::HomebrewCleaner;
#[cfg(target_os = "macos")]
pub use developer::XcodeCleaner;
pub use developer::{CargoCleaner, DockerCleaner, NpmCleaner, PipCleaner, YarnCleaner};
pub use dynamic::DynamicCleaner;
pub use system::{SystemCachesCleaner, SystemLogsCleaner, TrashCleaner};
