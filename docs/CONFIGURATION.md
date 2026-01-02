# Configuration Guide

CleanMac can be configured via a TOML file at `~/.config/cleanmac/config.toml`.

## Quick Start

```bash
# Create default config
cleanmac config init

# Edit with your $EDITOR
cleanmac config edit

# Show current config
cleanmac config show

# Validate config
cleanmac config validate
```

## Full Configuration Reference

```toml
# ~/.config/cleanmac/config.toml

[general]
# Default to dry-run mode (recommended: true)
dry_run_default = true

# Keep files newer than this many days (0 = clean all)
keep_recent_days = 0

# Output format: "table", "json", or "plain"
output_format = "table"

# Enable colored output
color = true

# Verbosity level (0 = quiet, 1 = normal, 2 = verbose, 3 = debug)
verbosity = 1

[safety]
# Show warning when cleaning more than this (bytes)
warn_threshold = 10737418240  # 10GB

# Require --force when cleaning more than this (bytes)
force_threshold = 53687091200  # 50GB

# Always ask for confirmation
always_confirm = true

# Enable audit logging
audit_enabled = true

# Keep audit logs for this many days
audit_retention_days = 90

# Additional protected paths (beyond hardcoded ones)
# protected_paths = [
#     "~/Projects",
#     "~/Work",
# ]

# Global exclusions (never clean these)
# exclusions = [
#     "~/Library/Caches/com.apple.Safari",
# ]

# ============================================================
# Category-specific configuration
# ============================================================

[categories.system_cache]
enabled = true
# Paths to exclude from cleaning
# exclude = ["~/Library/Caches/com.apple.Safari"]

[categories.system_logs]
enabled = true

[categories.trash]
enabled = true
# Only clean items older than this many days
min_age_days = 0

[categories.xcode]
enabled = true
# Keep archives newer than this many days
keep_recent_archives_days = 0
# iOS versions to always keep
keep_ios_versions = []  # e.g., ["17.0", "17.1"]

[categories.homebrew]
enabled = true
# Number of old versions to keep per formula
keep_versions = 0

[categories.npm]
enabled = true

[categories.yarn]
enabled = true

[categories.cargo]
enabled = true
# Keep crates used in the last N days
keep_recent_days = 0

[categories.pip]
enabled = true

[categories.docker]
enabled = true
# Only clean dangling images (safer)
dangling_only = true
# Clean unused volumes (CAUTION: may contain data!)
clean_volumes = false
# Clean build cache
clean_build_cache = true

[categories.apps]
enabled = true
# Bundle IDs of known uninstalled apps to look for
# known_apps = ["com.example.uninstalled-app"]
```

## Common Configurations

### Conservative (Maximum Safety)

```toml
[general]
dry_run_default = true
keep_recent_days = 30

[safety]
always_confirm = true
warn_threshold = 5368709120  # 5GB
force_threshold = 10737418240  # 10GB

[categories.trash]
min_age_days = 30

[categories.docker]
dangling_only = true
clean_volumes = false
```

### Developer Workstation

```toml
[general]
keep_recent_days = 7

[categories.xcode]
keep_recent_archives_days = 90
keep_ios_versions = ["17.0"]

[categories.homebrew]
keep_versions = 1

[categories.cargo]
keep_recent_days = 30
```

### Aggressive Cleaning

```toml
[general]
keep_recent_days = 0

[safety]
always_confirm = false

[categories.trash]
min_age_days = 0

[categories.docker]
dangling_only = false
clean_volumes = true
clean_build_cache = true
```

## Environment Variables

- `CLEANMAC_CONFIG` - Override config file path
- `NO_COLOR` - Disable colored output
- `RUST_LOG` - Set log level (e.g., `RUST_LOG=debug`)
