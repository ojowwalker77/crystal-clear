//! FFI module for Swift bindings via UniFFI.
//!
//! This module provides Swift-compatible types and functions for the Crystal Clear
//! native macOS application. All business logic remains in Rust - this module only
//! handles type conversion and FFI boundaries.
//!
//! Uses UniFFI's proc-macro approach for simpler setup.
//! The scaffolding is set up in lib.rs.

mod conversions;
mod core;
mod types;

pub use core::*;
pub use types::*;
