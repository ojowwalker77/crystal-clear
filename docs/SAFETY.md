# Safety Architecture

CleanMac is designed with multiple layers of safety to prevent accidental data loss.

## Core Principles

1. **Never permanently delete** - All files are moved to `~/.Trash`
2. **Dry-run by default** - Must explicitly pass `--execute` to clean
3. **Whitelist, not blacklist** - Only known-safe paths can be cleaned
4. **Defense in depth** - Multiple safety checks at different levels

## Safety Layers

### Layer 1: Protected Paths (Hardcoded)

These paths can NEVER be cleaned, regardless of configuration:

```
System Paths:
/                    /System              /Library
/Applications        /Users               /bin
/sbin                /usr                 /var
/etc                 /tmp                 /private
/cores               /opt                 /Volumes

User Paths:
~/Documents          ~/Desktop
~/Movies             ~/Music              ~/Pictures
~/Public             ~/.ssh               ~/.gnupg
~/.aws               ~/.kube
~/Library/Keychains  ~/Library/Mail
~/Library/Messages   ~/Library/Safari
~/Library/Calendars  ~/Library/Contacts
```

These are defined in `src/safety/protected_paths.rs` and cannot be overridden.

### Layer 2: Safe Boundaries

Each cleaner has a defined boundary within which it can operate:

```rust
// Example: Xcode cleaner can only operate within these paths
SafeBoundary::new(vec![
    "~/Library/Developer/Xcode/DerivedData",
    "~/Library/Developer/Xcode/Archives",
    "~/Library/Developer/Xcode/iOS DeviceSupport",
], "Xcode directories")
```

If a path (or its resolved symlink target) is outside the boundary, the operation fails.

### Layer 3: Symlink Validation

Before any operation, symlinks are resolved and validated:

1. The symlink target must exist
2. The resolved path must be within the cleaner's boundary
3. Symlink loops are detected and rejected

This prevents attacks where a malicious symlink points to a protected location.

### Layer 4: Path Validation

Every path is validated before any operation:

```rust
pub enum ValidationResult {
    Safe,                          // OK to clean
    HardProtected { reason },      // NEVER clean
    UserExcluded,                  // User excluded in config
    BoundaryViolation { reason },  // Outside safe boundary
    NotFound,                      // Path doesn't exist
    PermissionDenied,              // No access
}
```

### Layer 5: Audit Logging

All operations are logged to `~/.local/share/cleanmac/audit.log`:

```json
{
  "id": "uuid",
  "timestamp": "2024-01-15T10:30:00Z",
  "operation": "MoveToTrash",
  "cleaner_id": "xcode",
  "path": "/Users/user/Library/Developer/Xcode/DerivedData/MyProject",
  "size": 1073741824,
  "dry_run": false,
  "success": true
}
```

### Layer 6: Size Limits

- **Warning threshold**: 10GB - User is warned but can proceed
- **Force threshold**: 50GB - Requires `--force` flag to proceed

### Layer 7: Confirmation Prompts

Interactive confirmation is required before cleaning (unless `--yes` is passed).

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Safety violation |
| 3 | Configuration error |
| 4 | User cancelled |
| 5 | Partial success |

## Testing Safety

Run the safety tests:

```bash
cargo test safety
```

These tests verify that protected paths are never accessible to cleaners.
