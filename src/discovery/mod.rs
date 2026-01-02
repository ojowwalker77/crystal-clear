//! Dynamic path discovery for cleanable directories.

pub mod heuristics;
pub mod scanner;

pub use scanner::{DiscoveryConfig, PathDiscovery};
