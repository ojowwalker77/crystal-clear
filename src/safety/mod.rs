//! Safety module for CleanMac.
//!
//! This module provides the critical safety infrastructure that prevents
//! CleanMac from accidentally damaging the system. It includes:
//!
//! - Protected paths: Hardcoded list of paths that can NEVER be modified
//! - Boundaries: Defined safe zones where cleaners can operate
//! - Validators: Path validation including symlink escape detection

pub mod boundaries;
pub mod protected_paths;
pub mod validator;

pub use boundaries::SafeBoundary;
pub use protected_paths::{get_protected_paths, is_protected, matches_protected_pattern};
pub use validator::{PathValidator, ValidationResult};
