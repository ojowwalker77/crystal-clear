//! Developer tool cleaners.
//!
//! Platform-specific cleaners:
//! - Xcode: macOS only
//! - Homebrew: macOS and Linux

pub mod cargo;
pub mod docker;
pub mod npm;
pub mod pip;
pub mod yarn;

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub mod homebrew;

#[cfg(target_os = "macos")]
pub mod xcode;

// Cross-platform cleaners
pub use cargo::CargoCleaner;
pub use docker::DockerCleaner;
pub use npm::NpmCleaner;
pub use pip::PipCleaner;
pub use yarn::YarnCleaner;

// Platform-specific cleaners
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub use homebrew::HomebrewCleaner;

#[cfg(target_os = "macos")]
pub use xcode::XcodeCleaner;
