//! Database model types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Discovered path category.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PathCategory {
    Cache,
    Logs,
    Temp,
    Build,
    Deps,
    Archive,
    Other,
}

impl PathCategory {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "cache" => Self::Cache,
            "logs" => Self::Logs,
            "temp" => Self::Temp,
            "build" => Self::Build,
            "deps" => Self::Deps,
            "archive" => Self::Archive,
            _ => Self::Other,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cache => "cache",
            Self::Logs => "logs",
            Self::Temp => "temp",
            Self::Build => "build",
            Self::Deps => "deps",
            Self::Archive => "archive",
            Self::Other => "other",
        }
    }
}

/// A discovered path record.
#[derive(Debug, Clone)]
pub struct DiscoveredPath {
    pub id: Option<i64>,
    pub path: PathBuf,
    pub category: PathCategory,
    pub discovery_method: String,
    pub confidence: f64,
    pub last_seen: DateTime<Utc>,
    pub first_seen: DateTime<Utc>,
    pub last_size: Option<u64>,
    pub times_cleaned: u32,
    pub is_active: bool,
}

/// Discovery pattern for finding cleanable paths.
#[derive(Debug, Clone)]
pub struct DiscoveryPattern {
    pub id: Option<i64>,
    pub pattern: String,
    pub category: PathCategory,
    pub pattern_type: PatternType,
    pub priority: i32,
    pub enabled: bool,
    pub description: Option<String>,
}

/// Pattern matching type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternType {
    Glob,
    Regex,
    NameContains,
}

impl PatternType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "glob" => Self::Glob,
            "regex" => Self::Regex,
            _ => Self::NameContains,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Glob => "glob",
            Self::Regex => "regex",
            Self::NameContains => "name_contains",
        }
    }
}

/// Scan history record.
#[derive(Debug, Clone)]
pub struct ScanRecord {
    pub id: Option<i64>,
    pub scan_id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub scan_type: ScanType,
    pub total_items: Option<u32>,
    pub total_size: Option<u64>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
}

/// Type of scan performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanType {
    Full,
    Incremental,
}

impl ScanType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Incremental => "incremental",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "full" => Self::Full,
            _ => Self::Incremental,
        }
    }
}
