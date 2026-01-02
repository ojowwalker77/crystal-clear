//! Error handling module for CleanMac.

mod types;

pub use types::exit_codes;
pub use types::CleanmacError;

/// Result type alias for CleanMac operations.
pub type Result<T> = std::result::Result<T, CleanmacError>;
