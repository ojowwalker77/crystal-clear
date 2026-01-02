//! Trash module for CleanMac.
//!
//! This module handles moving files to the macOS trash.
//! IMPORTANT: Files are ONLY moved to trash, NEVER permanently deleted.

pub mod mover;

pub use mover::TrashMover;
