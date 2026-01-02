//! System cleaners (caches, logs, trash).

pub mod caches;
pub mod logs;
pub mod trash;

pub use caches::SystemCachesCleaner;
pub use logs::SystemLogsCleaner;
pub use trash::TrashCleaner;
