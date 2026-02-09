//! Docker cleaner.

use std::process::Command;
use std::time::Instant;

use crate::audit::AuditLogger;
use crate::cleaners::traits::*;
use crate::config::schema::DockerConfig;
use crate::safety::SafeBoundary;

/// Cleaner for Docker images, containers, and build cache.
#[allow(dead_code)]
pub struct DockerCleaner {
    audit_logger: AuditLogger,
    dangling_only: bool,
    clean_volumes: bool,
    clean_build_cache: bool,
}

impl DockerCleaner {
    pub fn new(audit_logger: AuditLogger, config: &DockerConfig) -> Self {
        Self {
            audit_logger,
            dangling_only: config.dangling_only,
            clean_volumes: config.clean_volumes,
            clean_build_cache: config.clean_build_cache,
        }
    }

    fn is_docker_running() -> bool {
        Command::new("docker")
            .args(["info"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn get_docker_disk_usage() -> Option<u64> {
        let output = Command::new("docker")
            .args(["system", "df", "--format", "{{.Size}}"])
            .output()
            .ok()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Parse the output (rough estimate)
            let mut total = 0u64;
            for line in stdout.lines() {
                // Docker reports sizes like "1.5GB", "500MB", etc.
                if let Some(size) = parse_docker_size(line.trim()) {
                    total += size;
                }
            }
            Some(total)
        } else {
            None
        }
    }
}

fn parse_docker_size(s: &str) -> Option<u64> {
    let s = s.trim().to_uppercase();

    if s == "0B" || s.is_empty() {
        return Some(0);
    }

    let (num_str, multiplier) = if s.ends_with("TB") {
        (&s[..s.len() - 2], 1024u64 * 1024 * 1024 * 1024)
    } else if s.ends_with("GB") {
        (&s[..s.len() - 2], 1024u64 * 1024 * 1024)
    } else if s.ends_with("MB") {
        (&s[..s.len() - 2], 1024u64 * 1024)
    } else if s.ends_with("KB") {
        (&s[..s.len() - 2], 1024u64)
    } else if s.ends_with("B") {
        (&s[..s.len() - 1], 1u64)
    } else {
        return None;
    };

    num_str
        .trim()
        .parse::<f64>()
        .ok()
        .map(|n| (n * multiplier as f64) as u64)
}

impl Cleaner for DockerCleaner {
    fn id(&self) -> &'static str {
        "docker"
    }

    fn name(&self) -> &'static str {
        "Docker"
    }

    fn description(&self) -> &'static str {
        "Docker images, containers, volumes, and build cache"
    }

    fn category(&self) -> CleanerCategory {
        CleanerCategory::Developer
    }

    fn safe_boundary(&self) -> SafeBoundary {
        // Docker manages its own storage
        SafeBoundary::new(vec![], "Docker managed storage")
    }

    fn is_available(&self) -> bool {
        Self::is_docker_running()
    }

    fn scan(&self, _ctx: &CleanerContext) -> ScanResult {
        let start = Instant::now();

        if !Self::is_docker_running() {
            return ScanResult {
                category: self.id().to_string(),
                items: Vec::new(),
                total_size: 0,
                scan_duration: start.elapsed(),
                errors: vec!["Docker is not running".to_string()],
            };
        }

        let mut items = Vec::new();

        // Get dangling images
        if let Ok(output) = Command::new("docker")
            .args(["images", "-f", "dangling=true", "-q"])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if !line.trim().is_empty() {
                        items.push(CleanableItem {
                            path: std::path::PathBuf::from(format!("docker:image:{}", line.trim())),
                            size: 0,
                            item_type: ItemType::File,
                            age_days: None,
                            description: format!("Dangling image: {}", line.trim()),
                            requires_force: false,
                            risk_level: RiskLevel::Low,
                        });
                    }
                }
            }
        }

        // Get unused images (if not dangling_only)
        if !self.dangling_only {
            if let Ok(output) = Command::new("docker").args(["images", "-q"]).output() {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    for line in stdout.lines() {
                        if !line.trim().is_empty() {
                            // Check if already added
                            let path =
                                std::path::PathBuf::from(format!("docker:image:{}", line.trim()));
                            if !items.iter().any(|i| i.path == path) {
                                items.push(CleanableItem {
                                    path,
                                    size: 0,
                                    item_type: ItemType::File,
                                    age_days: None,
                                    description: format!("Unused image: {}", line.trim()),
                                    requires_force: true, // Require force for non-dangling
                                    risk_level: RiskLevel::Medium,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Get build cache
        if self.clean_build_cache {
            items.push(CleanableItem {
                path: std::path::PathBuf::from("docker:buildcache"),
                size: 0,
                item_type: ItemType::Directory,
                age_days: None,
                description: "Docker build cache".to_string(),
                requires_force: false,
                risk_level: RiskLevel::Low,
            });
        }

        // Get volumes (if enabled)
        if self.clean_volumes {
            if let Ok(output) = Command::new("docker")
                .args(["volume", "ls", "-f", "dangling=true", "-q"])
                .output()
            {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    for line in stdout.lines() {
                        if !line.trim().is_empty() {
                            items.push(CleanableItem {
                                path: std::path::PathBuf::from(format!(
                                    "docker:volume:{}",
                                    line.trim()
                                )),
                                size: 0,
                                item_type: ItemType::Directory,
                                age_days: None,
                                description: format!("Dangling volume: {}", line.trim()),
                                requires_force: true, // Volumes may contain data!
                                risk_level: RiskLevel::High,
                            });
                        }
                    }
                }
            }
        }

        let total_size = Self::get_docker_disk_usage().unwrap_or(0);

        ScanResult {
            category: self.id().to_string(),
            items,
            total_size,
            scan_duration: start.elapsed(),
            errors: Vec::new(),
        }
    }

    fn clean(&self, items: &[CleanableItem], ctx: &CleanerContext) -> CleanResult {
        let start = Instant::now();
        let mut cleaned = 0;
        let mut failed = Vec::new();

        if ctx.dry_run {
            for item in items {
                let _ = self
                    .audit_logger
                    .log_dry_run(self.id(), &item.path, item.size);
            }
            return CleanResult {
                category: self.id().to_string(),
                items_cleaned: items.len(),
                bytes_freed: 0,
                items_failed: Vec::new(),
                clean_duration: start.elapsed(),
            };
        }

        for item in items {
            if item.requires_force && !ctx.force {
                failed.push((item.path.clone(), "Requires --force flag".to_string()));
                continue;
            }

            let path_str = item.path.to_string_lossy();

            let result = if path_str.starts_with("docker:image:") {
                let id = &path_str["docker:image:".len()..];
                Command::new("docker").args(["rmi", id]).output()
            } else if path_str.starts_with("docker:volume:") {
                let id = &path_str["docker:volume:".len()..];
                Command::new("docker").args(["volume", "rm", id]).output()
            } else if path_str == "docker:buildcache" {
                Command::new("docker")
                    .args(["builder", "prune", "-f"])
                    .output()
            } else {
                continue;
            };

            match result {
                Ok(o) if o.status.success() => {
                    let _ = self
                        .audit_logger
                        .log_cleaned(self.id(), &item.path, item.size);
                    cleaned += 1;
                }
                Ok(o) => {
                    let error_msg = String::from_utf8_lossy(&o.stderr).to_string();
                    let _ = self
                        .audit_logger
                        .log_error(self.id(), &item.path, &error_msg);
                    failed.push((item.path.clone(), error_msg));
                }
                Err(e) => {
                    let _ = self
                        .audit_logger
                        .log_error(self.id(), &item.path, &e.to_string());
                    failed.push((item.path.clone(), e.to_string()));
                }
            }
        }

        CleanResult {
            category: self.id().to_string(),
            items_cleaned: cleaned,
            bytes_freed: 0,
            items_failed: failed,
            clean_duration: start.elapsed(),
        }
    }
}
